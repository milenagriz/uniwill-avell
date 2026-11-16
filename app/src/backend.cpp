#include "backend.h"

#include <QDBusError>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QJsonDocument>
#include <QJsonObject>

static const char *const kPath = "/org/avell/Control1";
static const char *const kInterface = "org.avell.Control1";

// The errors that mean nobody answered, rather than that the daemon said no.
static bool unreachable(const QDBusError &error) {
    switch (error.type()) {
    case QDBusError::ServiceUnknown:
    case QDBusError::NoReply:
    case QDBusError::Disconnected:
    case QDBusError::NoServer:
    case QDBusError::Timeout:
    case QDBusError::TimedOut:
        return true;
    default:
        return error.name() == "org.freedesktop.DBus.Error.NameHasNoOwner";
    }
}

Backend::Backend(const QDBusConnection &bus, const QString &service, QObject *parent)
    : QObject(parent), m_bus(bus), m_service(service),
      m_watcher(service, bus, QDBusServiceWatcher::WatchForOwnerChange) {
    // The poll notices a stopped or restarted daemon within one interval; the
    // watcher makes it immediate.
    connect(&m_watcher, &QDBusServiceWatcher::serviceRegistered, this, &Backend::refresh);
    connect(&m_watcher, &QDBusServiceWatcher::serviceUnregistered, this, [this] { setRunning(false); });
    m_poll.setInterval(2000);
    connect(&m_poll, &QTimer::timeout, this, &Backend::refresh);
    m_poll.start();
    refresh();
}

void Backend::refresh() {
    auto call = [this](const char *method) {
        return new QDBusPendingCallWatcher(
            m_bus.asyncCall(QDBusMessage::createMethodCall(m_service, kPath, kInterface, method), 5000), this);
    };

    connect(call("GetStatus"), &QDBusPendingCallWatcher::finished, this, [this](QDBusPendingCallWatcher *watcher) {
        watcher->deleteLater();
        const QDBusPendingReply<QString> reply = *watcher;
        if (reply.isError()) {
            if (unreachable(reply.error()))
                setRunning(false);
            return;
        }
        m_status = QJsonDocument::fromJson(reply.value().toUtf8()).object().toVariantMap();
        setRunning(true);
        // Every poll, changed or not, so a control the owner moved goes back
        // to the machine's real state when the daemon did not follow it.
        emit statusChanged();
    });

    const int saves = m_saves;
    connect(call("GetConfigJson"), &QDBusPendingCallWatcher::finished, this, [this, saves](QDBusPendingCallWatcher *watcher) {
        watcher->deleteLater();
        const QDBusPendingReply<QString> reply = *watcher;
        if (!reply.isError() && saves == m_saves)
            adoptConfig(reply.value());
    });
}

void Backend::adoptConfig(const QString &json) {
    if (json == m_configJson)
        return;
    m_configJson = json;
    m_config = QJsonDocument::fromJson(json.toUtf8()).object().toVariantMap();
    emit configChanged();
}

void Backend::saveConfig(const QString &json) {
    const int saves = ++m_saves;
    auto message = QDBusMessage::createMethodCall(m_service, kPath, kInterface, "SetConfigJson");
    message << json;
    auto *watcher = new QDBusPendingCallWatcher(m_bus.asyncCall(message, 10000), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, saves](QDBusPendingCallWatcher *watcher) {
        watcher->deleteLater();
        const QDBusPendingReply<> reply = *watcher;
        if (reply.isError()) {
            if (unreachable(reply.error()))
                setRunning(false);
            // The daemon's own words: "invalid configuration: <reason>".
            emit configRejected(reply.error().message());
            return;
        }
        // Read back what the daemon now has in force, then say it is saved.
        auto *read = new QDBusPendingCallWatcher(
            m_bus.asyncCall(QDBusMessage::createMethodCall(m_service, kPath, kInterface, "GetConfigJson"), 5000), this);
        connect(read, &QDBusPendingCallWatcher::finished, this, [this, saves](QDBusPendingCallWatcher *read) {
            read->deleteLater();
            const QDBusPendingReply<QString> reply = *read;
            if (!reply.isError() && saves == m_saves)
                adoptConfig(reply.value());
            emit configSaved();
            refresh();
        });
    });
}

void Backend::set(const QString &method, const QVariantList &arguments) {
    auto message = QDBusMessage::createMethodCall(m_service, kPath, kInterface, method);
    message.setArguments(arguments);
    auto *watcher = new QDBusPendingCallWatcher(m_bus.asyncCall(message, 10000), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this](QDBusPendingCallWatcher *watcher) {
        watcher->deleteLater();
        const QDBusPendingReply<> reply = *watcher;
        if (reply.isError())
            fail(reply.error());
        else
            setLastError({});
        refresh();
    });
}

void Backend::fail(const QDBusError &error) {
    if (unreachable(error))
        setRunning(false);
    else
        setLastError(error.message());
}

void Backend::setProfile(const QString &name) { set("SetProfile", {name}); }
void Backend::setFan(const QString &mode, int percent) { set("SetFan", {mode, QVariant::fromValue<uint>(qMax(0, percent))}); }
void Backend::setKeyboardBrightness(int level) { set("SetKeyboardBrightness", {QVariant::fromValue<uint>(qMax(0, level))}); }
void Backend::setChargingProfile(const QString &name) { set("SetChargingProfile", {name}); }
void Backend::setChargeCap(int percent) { set("SetChargeCap", {QVariant::fromValue<uint>(qMax(0, percent))}); }
void Backend::clearError() { setLastError({}); }

void Backend::setKeyboardColor(int red, int green, int blue) {
    auto byte = [](int v) { return QVariant::fromValue<uchar>(static_cast<uchar>(qBound(0, v, 255))); };
    set("SetKeyboardColor", {byte(red), byte(green), byte(blue)});
}

void Backend::setRunning(bool running) {
    if (running == m_running)
        return;
    m_running = running;
    if (!running) {
        // A stopped daemon reports nothing: stale readings would mislead.
        m_status.clear();
        emit statusChanged();
    }
    emit runningChanged();
}

void Backend::setLastError(const QString &message) {
    if (message == m_lastError)
        return;
    m_lastError = message;
    emit lastErrorChanged();
}

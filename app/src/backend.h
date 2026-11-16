#pragma once

#include <QDBusConnection>
#include <QDBusServiceWatcher>
#include <QObject>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>

// The one client of avelld's control API. It holds no rules of its own: it
// reads the status and the configuration on a timer, passes every change to
// the daemon, and reports what the daemon answered.
class Backend : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY runningChanged)
    Q_PROPERTY(QVariantMap status READ status NOTIFY statusChanged)
    Q_PROPERTY(QVariantMap config READ config NOTIFY configChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

public:
    explicit Backend(const QDBusConnection &bus = QDBusConnection::systemBus(),
                     const QString &service = "org.avell.Control1", QObject *parent = nullptr);

    bool running() const { return m_running; }
    QVariantMap status() const { return m_status; }
    QVariantMap config() const { return m_config; }
    QString lastError() const { return m_lastError; }

    Q_INVOKABLE void refresh();
    // The whole configuration as JSON text. The daemon judges it: configSaved()
    // or configRejected() with the daemon's own reason follows.
    Q_INVOKABLE void saveConfig(const QString &json);
    Q_INVOKABLE void setProfile(const QString &name);
    Q_INVOKABLE void setFan(const QString &mode, int percent = 0);
    Q_INVOKABLE void setKeyboardBrightness(int level);
    Q_INVOKABLE void setKeyboardColor(int red, int green, int blue);
    Q_INVOKABLE void setChargingProfile(const QString &name);
    Q_INVOKABLE void setChargeCap(int percent);
    Q_INVOKABLE void clearError();

signals:
    void runningChanged();
    void statusChanged();
    void configChanged();
    void lastErrorChanged();
    void configSaved();
    void configRejected(const QString &reason);

private:
    void set(const QString &method, const QVariantList &arguments);
    void fail(const class QDBusError &error);
    void setRunning(bool running);
    void setLastError(const QString &message);
    void adoptConfig(const QString &json);

    QDBusConnection m_bus;
    QString m_service;
    QDBusServiceWatcher m_watcher;
    QTimer m_poll;
    bool m_running = false;
    QVariantMap m_status, m_config;
    QString m_configJson, m_lastError;
    // Counts saves, so a configuration read that was under way before a save
    // cannot put the old values back.
    int m_saves = 0;
};

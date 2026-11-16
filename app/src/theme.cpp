// Theme watching follows Omacut (MIT, David Heinemeier Hansson).
#include "theme.h"

#include <QColor>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>
#include <QStandardPaths>
#include <cmath>

// A colour between two others, for the tones the theme file does not name.
static QString mix(const QString &from, const QString &to, double share) {
    const QColor a(from), b(to);
    return QColor::fromRgbF(a.redF() + (b.redF() - a.redF()) * share, a.greenF() + (b.greenF() - a.greenF()) * share,
                            a.blueF() + (b.blueF() - a.blueF()) * share).name();
}

Theme::Theme(const QString &directory, QObject *parent, const QString &hyprctl)
    : QObject(parent),
      m_directory(directory.isEmpty() ? QDir::homePath() + "/.local/state/omarchy/current" : directory) {
    m_debounce.setSingleShot(true);
    m_debounce.setInterval(80);
    connect(&m_watcher, &QFileSystemWatcher::directoryChanged, this, [this] { m_debounce.start(); });
    connect(&m_watcher, &QFileSystemWatcher::fileChanged, this, [this] { m_debounce.start(); });
    connect(&m_debounce, &QTimer::timeout, this, &Theme::reload);

    // Corners follow Hyprland's effective rounding, which themes and personal
    // overrides can both set, so ask hyprctl rather than reading its config.
    // Without Hyprland the corners stay sharp, like Omarchy's default.
    m_hyprctl = !hyprctl.isEmpty() ? hyprctl
              : directory.isEmpty() && !qEnvironmentVariableIsEmpty("HYPRLAND_INSTANCE_SIGNATURE")
                  ? QStandardPaths::findExecutable("hyprctl") : QString();
    m_roundingTimeout.setSingleShot(true);
    m_roundingTimeout.setInterval(500);
    connect(&m_roundingTimeout, &QTimer::timeout, &m_roundingProcess, &QProcess::kill);
    connect(&m_roundingProcess, &QProcess::errorOccurred, this, [this] { m_roundingTimeout.stop(); });
    connect(&m_roundingProcess, &QProcess::finished, this, [this](int code, QProcess::ExitStatus status) {
        m_roundingTimeout.stop();
        const auto value = QJsonDocument::fromJson(m_roundingProcess.readAllStandardOutput()).object().value("int");
        if (code != 0 || status != QProcess::NormalExit || !value.isDouble() || value.toInt(-1) < 0)
            return;
        if (value.toInt() != m_radius) {
            m_radius = value.toInt();
            emit changed();
        }
    });
    m_roundingPoll.setInterval(1000);
    connect(&m_roundingPoll, &QTimer::timeout, this, &Theme::refreshRounding);
    if (!m_hyprctl.isEmpty())
        m_roundingPoll.start();
    reload();
}

Theme::~Theme() {
    if (m_roundingProcess.state() != QProcess::NotRunning) {
        m_roundingProcess.kill();
        m_roundingProcess.waitForFinished(1000);
    }
}

void Theme::refreshRounding() {
    if (m_hyprctl.isEmpty() || m_roundingProcess.state() != QProcess::NotRunning)
        return;
    m_roundingProcess.start(m_hyprctl, {"-j", "getoption", "decoration:rounding"});
    m_roundingTimeout.start();
}

QHash<QString, QString> Theme::readColors(const QString &path, bool *dark) {
    QHash<QString, QString> colors;
    if (dark)
        *dark = true;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return colors;
    const QString text = QString::fromUtf8(file.readAll());
    static const QRegularExpression color(R"re(^\s*([a-z_]+)\s*=\s*["'](#[0-9a-fA-F]{6})["'])re",
                                          QRegularExpression::MultilineOption);
    for (auto it = color.globalMatch(text); it.hasNext();) {
        const auto match = it.next();
        colors.insert(match.captured(1), match.captured(2));
    }
    static const QRegularExpression mode(R"re(^\s*mode\s*=\s*["']light["'])re", QRegularExpression::MultilineOption);
    if (dark && mode.match(text).hasMatch())
        *dark = false;
    return colors;
}

// Black or white, whichever reads on the accent (WCAG relative luminance).
QString Theme::accentForeground() const {
    const QColor c(accent());
    auto linear = [](double v) { return v <= .04045 ? v / 12.92 : std::pow((v + .055) / 1.055, 2.4); };
    const double luminance = .2126 * linear(c.redF()) + .7152 * linear(c.greenF()) + .0722 * linear(c.blueF());
    return luminance > .179 ? "black" : "white";
}

// The panels' colour: the theme's lighter background when it differs from
// the window's, else a step from the background towards the text.
QString Theme::surface() const {
    const QString lighter = m_colors.value("lighter_background");
    if (!lighter.isEmpty() && QColor(lighter) != QColor(background()))
        return lighter;
    return mix(background(), foreground(), .07);
}

// Secondary text. Some themes' "muted" is too dim to read on the background,
// so it is always a readable step between the two.
QString Theme::muted() const { return mix(background(), foreground(), .62); }

QString Theme::border() const { return mix(background(), foreground(), .16); }

void Theme::reload() {
    refreshRounding();
    const auto paths = m_watcher.files() + m_watcher.directories();
    if (!paths.isEmpty())
        m_watcher.removePaths(paths);

    // A theme switch replaces symlinks and files, so watch the parents too and
    // re-arm after every change. Watching the nearest existing ancestor means
    // installing Omarchy later works without restarting the app.
    QString ancestor = m_directory;
    while (!QFileInfo::exists(ancestor) && ancestor != "/")
        ancestor = QFileInfo(ancestor).absolutePath();
    QStringList candidates{ancestor, QFileInfo(ancestor).absolutePath(), m_directory,
                           m_directory + "/theme", m_directory + "/theme/colors.toml"};
    candidates.removeDuplicates();
    for (const auto &path : candidates)
        if (QFileInfo::exists(path))
            m_watcher.addPath(path);

    bool dark = true;
    const auto colors = readColors(m_directory + "/theme/colors.toml", &dark);
    if (colors != m_colors || dark != m_dark) {
        m_colors = colors;
        m_dark = dark;
        emit changed();
    }
}

#pragma once

#include <QFileSystemWatcher>
#include <QHash>
#include <QObject>
#include <QProcess>
#include <QTimer>

// The Omarchy theme, followed live: the colours of the current theme's
// colors.toml and Hyprland's window rounding.
class Theme : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool dark READ dark NOTIFY changed)
    Q_PROPERTY(QString accent READ accent NOTIFY changed)
    Q_PROPERTY(QString accentForeground READ accentForeground NOTIFY changed)
    Q_PROPERTY(QString background READ background NOTIFY changed)
    Q_PROPERTY(QString surface READ surface NOTIFY changed)
    Q_PROPERTY(QString foreground READ foreground NOTIFY changed)
    Q_PROPERTY(QString muted READ muted NOTIFY changed)
    Q_PROPERTY(QString border READ border NOTIFY changed)
    Q_PROPERTY(QString red READ red NOTIFY changed)
    Q_PROPERTY(QString yellow READ yellow NOTIFY changed)
    Q_PROPERTY(QString green READ green NOTIFY changed)
    Q_PROPERTY(int radius READ radius NOTIFY changed)

public:
    explicit Theme(const QString &currentDirectory = {}, QObject *parent = nullptr, const QString &hyprctl = {});
    ~Theme() override;

    bool dark() const { return m_dark; }
    QString accent() const { return color("accent", "#FFD60A"); }
    QString accentForeground() const;
    QString background() const { return color("background", m_dark ? "#0e0e10" : "#f4f4f5"); }
    QString surface() const;
    QString foreground() const { return color("foreground", m_dark ? "#e6e6e9" : "#1c1c1f"); }
    QString muted() const;
    QString border() const;
    QString red() const { return color("red", "#e0604e"); }
    QString yellow() const { return color("yellow", "#e8b65c"); }
    QString green() const { return color("green", "#8fb06a"); }
    int radius() const { return m_radius; }

    // The colours (key = "#rrggbb") and the mode of a colors.toml.
    static QHash<QString, QString> readColors(const QString &path, bool *dark = nullptr);

signals:
    void changed();

private:
    void reload();
    void refreshRounding();
    QString color(const char *key, const char *fallback) const { return m_colors.value(QLatin1String(key), QLatin1String(fallback)); }

    QString m_directory;
    QHash<QString, QString> m_colors;
    bool m_dark = true;
    QFileSystemWatcher m_watcher;
    QTimer m_debounce;
    QString m_hyprctl;
    int m_radius = 0;
    QProcess m_roundingProcess;
    QTimer m_roundingPoll, m_roundingTimeout;
};

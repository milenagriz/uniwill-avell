#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusContext>
#include <QDBusError>
#include <QDir>
#include <QFile>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>

#include "backend.h"
#include "theme.h"
#include "translator.h"

// A stand-in for avelld on the session bus: it answers the control API's
// methods from memory and refuses a configuration that says "bad".
class FakeDaemon : public QObject, protected QDBusContext {
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.avell.Control1")

public:
    QString config = R"({"fans":{"hysteresis_c":4}})";
    QString profile = "balanced";
    QList<int> color;

public slots:
    QString GetStatus() { return QString(R"({"profile":"%1","unavailable":["charge_cap"]})").arg(profile); }
    QString GetConfigJson() { return config; }
    void SetConfigJson(const QString &json) {
        if (json.contains("bad"))
            sendErrorReply(QDBusError::InvalidArgs, "invalid configuration: fans.hysteresis_c is 99: allowed are 0 to 20 °C");
        else
            config = json;
    }
    void SetProfile(const QString &name) { profile = name; }
    void SetChargeCap(uint) { sendErrorReply(QDBusError::Failed, "the charge cap is not exposed by the driver on this machine"); }
    void SetKeyboardColor(uchar red, uchar green, uchar blue) { color = {red, green, blue}; }
};

class AvellControlTests : public QObject {
    Q_OBJECT

private slots:
    void initTestCase()
    {
        if (!QDBusConnection::sessionBus().isConnected())
            QSKIP("These tests need a session bus.");
        m_service = QString("org.avell.ControlTest.p%1").arg(QCoreApplication::applicationPid());
        QVERIFY(QDBusConnection::sessionBus().registerObject("/org/avell/Control1", &m_daemon, QDBusConnection::ExportAllSlots));
    }

    void init() { QVERIFY(QDBusConnection::sessionBus().registerService(m_service)); }
    void cleanup() { QDBusConnection::sessionBus().unregisterService(m_service); }

    void readsTheStatusAndTheConfiguration()
    {
        Backend backend(QDBusConnection::sessionBus(), m_service);
        QVERIFY(!backend.running());
        QTRY_VERIFY(backend.running());
        QCOMPARE(backend.status().value("profile").toString(), QString("balanced"));
        QCOMPARE(backend.status().value("unavailable").toStringList(), QStringList{"charge_cap"});
        QTRY_COMPARE(backend.config().value("fans").toMap().value("hysteresis_c").toInt(), 4);
    }

    void savesAConfigurationTheDaemonAccepts()
    {
        Backend backend(QDBusConnection::sessionBus(), m_service);
        QSignalSpy saved(&backend, &Backend::configSaved), rejected(&backend, &Backend::configRejected);
        backend.saveConfig(R"({"fans":{"hysteresis_c":6}})");
        QTRY_COMPARE(saved.count(), 1);
        QCOMPARE(rejected.count(), 0);
        QCOMPARE(backend.config().value("fans").toMap().value("hysteresis_c").toInt(), 6);
        m_daemon.config = R"({"fans":{"hysteresis_c":4}})";
    }

    void reportsTheDaemonsReasonForRefusingAConfiguration()
    {
        Backend backend(QDBusConnection::sessionBus(), m_service);
        QTRY_VERIFY(!backend.config().isEmpty());
        QSignalSpy saved(&backend, &Backend::configSaved), rejected(&backend, &Backend::configRejected);
        backend.saveConfig(R"({"bad":true})");
        QTRY_COMPARE(rejected.count(), 1);
        QCOMPARE(rejected.first().first().toString(),
                 QString("invalid configuration: fans.hysteresis_c is 99: allowed are 0 to 20 °C"));
        QCOMPARE(saved.count(), 0);
        QCOMPARE(backend.config().value("fans").toMap().value("hysteresis_c").toInt(), 4);
        QVERIFY(backend.running());
    }

    void settersReachTheDaemonAndItsErrorsAreKept()
    {
        Backend backend(QDBusConnection::sessionBus(), m_service);
        backend.setProfile("performance");
        QTRY_COMPARE(backend.status().value("profile").toString(), QString("performance"));
        backend.setKeyboardColor(255, 128, 0);
        QTRY_COMPARE(m_daemon.color, (QList<int>{255, 128, 0}));
        QCOMPARE(backend.lastError(), QString());

        backend.setChargeCap(80);
        QTRY_COMPARE(backend.lastError(), QString("the charge cap is not exposed by the driver on this machine"));
        QVERIFY(backend.running());
        backend.clearError();
        QCOMPARE(backend.lastError(), QString());
        m_daemon.profile = "balanced";
    }

    void followsTheDaemonStoppingAndComingBack()
    {
        Backend backend(QDBusConnection::sessionBus(), m_service);
        QTRY_VERIFY(backend.running());
        QDBusConnection::sessionBus().unregisterService(m_service);
        QTRY_VERIFY(!backend.running());
        QVERIFY(backend.status().isEmpty());
        QVERIFY(QDBusConnection::sessionBus().registerService(m_service));
        QTRY_VERIFY(backend.running());
        QCOMPARE(backend.status().value("profile").toString(), QString("balanced"));
    }

    void themeFallsBack()
    {
        QTemporaryDir dir;
        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QString("#FFD60A"));
        QCOMPARE(theme.accentForeground(), QString("black"));
        QVERIFY(theme.dark());
        QCOMPARE(theme.background(), QString("#0e0e10"));
    }

    void themeReadsTheColours()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml",
              "mode = \"light\"\naccent = \"#112233\"\nbackground = \"#fafafa\"\nforeground = \"#101010\"\n");

        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QString("#112233"));
        QCOMPARE(theme.accentForeground(), QString("white"));
        QCOMPARE(theme.background(), QString("#fafafa"));
        QCOMPARE(theme.foreground(), QString("#101010"));
        QVERIFY(!theme.dark());
    }

    void themeFollowsAChange()
    {
        QTemporaryDir dir;
        QDir(dir.path()).mkpath("theme");
        write(dir.path() + "/theme/colors.toml", "accent = \"#112233\"\n");
        Theme theme(dir.path());
        QCOMPARE(theme.accent(), QString("#112233"));

        // As a theme switch does it: the file is replaced, not rewritten.
        QVERIFY(QFile::remove(dir.path() + "/theme/colors.toml"));
        write(dir.path() + "/theme/colors.toml", "accent = \"#445566\"\n");
        QTRY_COMPARE(theme.accent(), QString("#445566"));
    }

    void themeFollowsHyprlandRounding()
    {
        QTemporaryDir dir;
        const QString response = dir.filePath("response.json"), command = dir.filePath("hyprctl");
        write(command, "#!/bin/sh\ncat '" + response.toUtf8() + "'\n");
        QVERIFY(QFile::setPermissions(command, QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner));
        write(response, "{\"int\":0}");

        Theme theme(dir.path(), nullptr, command);
        QCOMPARE(theme.radius(), 0);
        write(response, "{\"int\":6}");
        QTRY_COMPARE(theme.radius(), 6);
    }

    void translatorFallsBackToEnglish()
    {
        QTemporaryDir dir;
        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("en"));
        QVERIFY(translator.translate("Main", "Save").isNull());
    }

    void translatorFollowsTheLanguageFile()
    {
        QTemporaryDir dir;
        write(dir.path() + "/language", "pt-BR\n");
        write(dir.path() + "/pt.json", R"({"Save": "Guardar", "Open": "Abrir"})");
        write(dir.path() + "/pt-BR.json", R"({"Save": "Salvar"})");

        Translator translator(dir.path() + "/language", dir.path());
        QCOMPARE(translator.language(), QString("pt-BR"));
        QCOMPARE(translator.translate("Main", "Save"), QString("Salvar"));
        QCOMPARE(translator.translate("Main", "Open"), QString("Abrir"));
        QVERIFY(translator.translate("Main", "Quit").isNull());
    }

private:
    static void write(const QString &path, const QByteArray &contents)
    {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(contents);
    }

    FakeDaemon m_daemon;
    QString m_service;
};

QTEST_MAIN(AvellControlTests)
#include "avell-control_tests.moc"

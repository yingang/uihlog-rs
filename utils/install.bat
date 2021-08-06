@echo off

set command="\"%~dp0uihlog.exe\" \"%%1\""

echo unUIHLog
reg add "HKEY_CURRENT_USER\Software\Classes\.uihlog" /ve /t REG_SZ /d "uihlog_auto_file" /f
reg add "HKEY_CURRENT_USER\Software\Classes\uihlog_auto_file\shell\unUIHLog" /ve /t REG_SZ /d "un-UIHLog" /f
reg add "HKEY_CURRENT_USER\Software\Classes\uihlog_auto_file\shell\unUIHLog\command" /ve /t REG_SZ /d %command% /f

echo unUIHlog
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog" /ve /t REG_SZ /d "un-UIHlog" /f
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog\command" /ve /t REG_SZ /d %command% /f

@echo off

set command="\"%~dp0uihlog.exe\" \"%%1\" 1"

echo unUIHlog
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog-PID" /ve /t REG_SZ /d "un-UIHlog-PID" /f
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog-PID\command" /ve /t REG_SZ /d %command% /f

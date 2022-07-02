@echo off

reg delete "HKEY_CURRENT_USER\Software\Classes\.uihlog" /f
reg delete "HKEY_CURRENT_USER\Software\Classes\uihlog_auto_file" /f
reg delete "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog" /f
reg delete "HKEY_CURRENT_USER\Software\Classes\Directory\shell\unUIHlog-PID" /f
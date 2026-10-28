@echo off
d run "%~dp0main.dev" -- cc %*
exit /b %ERRORLEVEL%

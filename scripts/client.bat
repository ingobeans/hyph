@echo off

REM Runs client and saves log

set SAVESTAMP=%DATE:/=-%@%TIME::=-%
set SAVESTAMP=%SAVESTAMP: =%
set SAVESTAMP=%SAVESTAMP:,=.%.log

mkdir logs > nul 2>&1
target\debug\client.exe > logs/%SAVESTAMP% 2>&1
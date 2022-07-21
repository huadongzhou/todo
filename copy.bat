@echo off

set filePath=%~dp0
echo %filePath%

echo %filePath%Download
cd %filePath%Download

echo %filePath%Download\app
copy "%filePath%"Download\app "%filePath%"resources\app.asar
echo 'copy success'

echo 'open exing'
cd %filePath%
sleep 3000
start Todoing.exe
echo 'open exed'
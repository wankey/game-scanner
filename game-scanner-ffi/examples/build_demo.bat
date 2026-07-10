@echo off
rem Helper to build the FFI demo against the just-built .lib / .dll.
rem Run from any shell inside the repo. Uses MSVC's vcvarsall.bat to
rem set up paths, then invokes cl.exe. Outputs:
rem   game-scanner-ffi\examples\ffi_demo.exe
rem Copy target\release\game_scanner_ffi.dll next to the exe before running.

setlocal
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat" x64 >nul
if errorlevel 1 (
  echo Failed to load MSVC environment.
  exit /b 1
)

pushd "%~dp0\..\.."

cl /nologo /std:c++17 /EHsc ^
   /I game-scanner-ffi\include ^
   game-scanner-ffi\examples\ffi_demo.cpp ^
   /Fe:game-scanner-ffi\examples\ffi_demo.exe ^
   /link /LIBPATH:target\release game_scanner_ffi.dll.lib

set RC=%ERRORLEVEL%
popd

copy /Y target\release\game_scanner_ffi.dll game-scanner-ffi\examples\ >nul

exit /b %RC%

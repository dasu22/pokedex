@echo off
cd /d "%~dp0"
echo Starting Pokedex Tracker...
echo Open http://localhost:3000 in your browser
echo Other devices on your network: http://<your-ip>:3000
echo Press Ctrl+C to stop.
target\release\pokedex.exe

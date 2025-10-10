#!/bin/bash

# Script to register NXM protocol with Crossover Mod Manager
# Run this after building and installing the app

APP_NAME="Crossover Mod Manager"
APP_PATH="/Applications/Crossover Mod Manager.app"
PROTOCOL="nxm"

echo "Registering NXM protocol with $APP_NAME..."

# Check if the app exists
if [ ! -d "$APP_PATH" ]; then
    echo "Error: $APP_PATH not found!"
    echo "Please build and install the app first using 'npm run tauri:build'"
    exit 1
fi

# Register the protocol using duti (if available)
if command -v duti &> /dev/null; then
    echo "Using duti to register protocol..."
    duti -s com.beneccles.crossover-mod-manager $PROTOCOL all
    echo "Protocol registered successfully!"
else
    echo "duti not found. Installing via Homebrew..."
    if command -v brew &> /dev/null; then
        brew install duti
        duti -s com.beneccles.crossover-mod-manager $PROTOCOL all
        echo "Protocol registered successfully!"
    else
        echo "Homebrew not found. Manual registration required:"
        echo "1. Right-click on any NXM link on NexusMods"
        echo "2. Select 'Choose Application...'"
        echo "3. Navigate to Applications and select 'Crossover Mod Manager'"
        echo "4. Check 'Always use this application'"
    fi
fi

echo "Done!"
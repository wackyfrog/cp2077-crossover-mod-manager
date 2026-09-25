# dmgbuild settings for the macOS disk image (see scripts/build-dmg.sh).
#
# The same window Tauri's bundle_dmg.sh lays out, written straight into the
# image's .DS_Store instead of asking Finder to arrange an open window — that
# AppleScript opened the volume as a tab of whatever Finder window was in
# front (with "prefer tabs" on) and resized it.
#
# Passed in with -D: app (path to the .app), icon (volume icon .icns).
import os.path

application = defines["app"]  # noqa: F821 — provided by dmgbuild
appname = os.path.basename(application)

format = "UDZO"
filesystem = "HFS+"

files = [application]
symlinks = {"Applications": "/Applications"}
hide_extensions = [appname]
icon = defines["icon"]  # noqa: F821

# Tauri's defaults: window at 10,60, 660x400; icons 128 with 16pt labels
window_rect = ((10, 60), (660, 400))
default_view = "icon-view"
show_toolbar = False
show_status_bar = False
show_sidebar = False
arrange_by = None
icon_size = 128
text_size = 16
icon_locations = {
    appname: (180, 170),
    "Applications": (480, 170),
}

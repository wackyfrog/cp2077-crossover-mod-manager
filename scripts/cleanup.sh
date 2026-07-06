#!/bin/bash
set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Resolve project root relative to this script (works from any cwd)
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
PROJECT_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
TARGET="$PROJECT_ROOT/src-tauri/target"

# Defaults
SCOPE="all"
DRY_RUN=0
ASSUME_YES=0

usage() {
    cat <<EOF
Cleanup Rust build cache (src-tauri/target).

Usage: ./scripts/cleanup.sh [scope] [options]

Scope (default: all):
  --all         Remove the whole target/ (frees debug + release)
  --debug       Remove only target/debug (dev build cache)
  --release     Remove only target/release (release build cache)

Options:
  -n, --dry-run Show what would be freed, delete nothing
  -y, --yes     Skip the confirmation prompt
  -h, --help    Show this help

Note: target/ is git-ignored and fully regenerable. The next build
      recompiles the cleaned profile from scratch (a few minutes).
EOF
}

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --all)     SCOPE="all" ;;
        --debug)   SCOPE="debug" ;;
        --release) SCOPE="release" ;;
        -n|--dry-run) DRY_RUN=1 ;;
        -y|--yes)     ASSUME_YES=1 ;;
        -h|--help)    usage; exit 0 ;;
        *)
            print_error "Unknown argument: $1"
            usage
            exit 1
            ;;
    esac
    shift
done

# Human-readable size of a path (prints "0B" if missing/empty)
dir_size() {
    if [ -d "$1" ]; then
        du -sh "$1" 2>/dev/null | cut -f1
    else
        echo "0B"
    fi
}

# Guard: never operate outside the expected target directory
if [ ! -d "$TARGET" ]; then
    print_success "Nothing to clean — $TARGET does not exist."
    exit 0
fi
case "$TARGET" in
    */src-tauri/target) ;;
    *)
        print_error "Refusing to clean unexpected path: $TARGET"
        exit 1
        ;;
esac

# Determine what will be removed
case "$SCOPE" in
    all)     TO_REMOVE="$TARGET";          LABEL="whole target/" ;;
    debug)   TO_REMOVE="$TARGET/debug";    LABEL="target/debug" ;;
    release) TO_REMOVE="$TARGET/release";  LABEL="target/release" ;;
esac

print_info "Project:  $PROJECT_ROOT"
print_info "Scope:    $LABEL"
print_info "Path:     $TO_REMOVE"

if [ ! -d "$TO_REMOVE" ]; then
    print_success "Nothing to clean — $TO_REMOVE does not exist."
    exit 0
fi

SIZE=$(dir_size "$TO_REMOVE")
print_info "Size:     $SIZE"
echo ""

if [ "$DRY_RUN" -eq 1 ]; then
    print_warning "Dry run — nothing was deleted. Would free $SIZE."
    exit 0
fi

# Confirm unless -y
if [ "$ASSUME_YES" -ne 1 ]; then
    print_warning "This will delete $LABEL and free ~$SIZE."
    read -p "Do you want to continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Cleanup cancelled"
        exit 0
    fi
fi

# Perform cleanup — prefer cargo clean where it maps cleanly, else rm -rf
print_info "Cleaning..."
if command -v cargo >/dev/null 2>&1 && [ -f "$PROJECT_ROOT/src-tauri/Cargo.toml" ]; then
    case "$SCOPE" in
        all)
            ( cd "$PROJECT_ROOT/src-tauri" && cargo clean )
            ;;
        release)
            ( cd "$PROJECT_ROOT/src-tauri" && cargo clean --release )
            ;;
        debug)
            # cargo has no "debug-only" clean; remove the profile dir directly
            rm -rf "$TO_REMOVE"
            ;;
    esac
else
    rm -rf "$TO_REMOVE"
fi

print_success "Cleanup complete — freed ~$SIZE"
print_info "Remaining target/: $(dir_size "$TARGET")"

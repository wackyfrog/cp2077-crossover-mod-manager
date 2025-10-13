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

# Check if version argument is provided
if [ -z "$1" ]; then
    print_error "Usage: ./scripts/release.sh <version>"
    print_info "Example: ./scripts/release.sh 1.7.0"
    exit 1
fi

VERSION=$1
TAG="v$VERSION"

# Validate version format (semantic versioning)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    print_error "Invalid version format. Use semantic versioning (e.g., 1.7.0)"
    exit 1
fi

print_info "Starting release process for version $VERSION"
echo ""

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_warning "You have uncommitted changes. Please commit or stash them first."
    git status --short
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
    print_warning "You are not on the main branch (current: $CURRENT_BRANCH)"
    read -p "Do you want to continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Pull latest changes
print_info "Pulling latest changes from remote..."
git pull origin main

# Update version in tauri.conf.json
print_info "Updating version in src-tauri/tauri.conf.json..."
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
rm src-tauri/tauri.conf.json.bak
print_success "Version updated in tauri.conf.json"

# Check if CHANGELOG.md has entry for this version
print_info "Checking CHANGELOG.md..."
if ! grep -q "## \[$VERSION\]" CHANGELOG.md; then
    print_warning "No changelog entry found for version $VERSION"
    print_info "Please add a changelog entry before continuing."
    print_info "Opening CHANGELOG.md..."
    
    # Add template to CHANGELOG if not exists
    TEMP_FILE=$(mktemp)
    echo "## [$VERSION] - $(date +%Y-%m-%d)" > "$TEMP_FILE"
    echo "" >> "$TEMP_FILE"
    echo "### Added" >> "$TEMP_FILE"
    echo "- New feature description" >> "$TEMP_FILE"
    echo "" >> "$TEMP_FILE"
    echo "### Changed" >> "$TEMP_FILE"
    echo "- Changed feature description" >> "$TEMP_FILE"
    echo "" >> "$TEMP_FILE"
    echo "### Fixed" >> "$TEMP_FILE"
    echo "- Bug fix description" >> "$TEMP_FILE"
    echo "" >> "$TEMP_FILE"
    cat CHANGELOG.md >> "$TEMP_FILE"
    mv "$TEMP_FILE" CHANGELOG.md
    
    ${EDITOR:-nano} CHANGELOG.md
    
    print_info "Please review and save the changelog, then run this script again."
    exit 1
fi
print_success "Changelog entry found for version $VERSION"

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    print_error "Tag $TAG already exists"
    print_info "If you want to recreate it, delete it first:"
    print_info "  git tag -d $TAG"
    print_info "  git push origin :refs/tags/$TAG"
    exit 1
fi

# Show changes to be committed
print_info "The following changes will be committed:"
git diff src-tauri/tauri.conf.json CHANGELOG.md

echo ""
print_warning "Ready to create release v$VERSION"
read -p "Do you want to continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Release cancelled"
    exit 0
fi

# Commit version changes
print_info "Committing version changes..."
git add src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: Release v$VERSION

- Update version to $VERSION
- Update CHANGELOG.md with release notes"
print_success "Changes committed"

# Create and push tag
print_info "Creating tag $TAG..."
git tag -a "$TAG" -m "Release v$VERSION"
print_success "Tag created"

# Push changes
print_info "Pushing changes to remote..."
git push origin main
print_success "Changes pushed to main"

print_info "Pushing tag to remote..."
git push origin "$TAG"
print_success "Tag pushed"

echo ""
print_success "Release process complete! 🎉"
print_info "GitHub Actions will now build and publish the release."
print_info "Monitor progress at: https://github.com/beneccles/crossover-mod-manager/actions"
print_info "Release will be available at: https://github.com/beneccles/crossover-mod-manager/releases/tag/$TAG"

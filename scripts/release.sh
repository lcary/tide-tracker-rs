#!/bin/bash
# Local release script for tide-tracker
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 1.2.3

set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.2.3"
    exit 1
fi

# Validate semantic version format
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semantic version format (e.g., 1.2.3)"
    exit 1
fi

echo "🚀 Preparing release v$VERSION"

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$CURRENT_BRANCH" != "main" ]]; then
    echo "Warning: You're not on the main branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo "Error: You have uncommitted changes"
    git status --short
    exit 1
fi

# Update version in Cargo.toml
echo "📝 Updating Cargo.toml version to $VERSION"
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm Cargo.toml.bak

# Update Cargo.lock
echo "🔄 Updating Cargo.lock"
cargo check > /dev/null

# Run tests to make sure everything works
echo "🧪 Running tests"
cargo test --all-features

# Build release binary to verify it compiles
echo "🔨 Building release binary"
cargo build --release

# Commit version bump
echo "📋 Committing version bump"
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to v$VERSION"

# Create and push tag
echo "🏷️  Creating and pushing tag v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"

echo "✅ Release v$VERSION prepared!"
echo ""
echo "To complete the release:"
echo "1. Push the commit: git push origin main"
echo "2. Push the tag: git push origin v$VERSION"
echo "3. GitHub Actions will automatically create the release and build binaries"
echo ""
echo "Or push both at once:"
echo "git push origin main && git push origin v$VERSION"

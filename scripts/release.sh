#!/bin/bash

# Release script for rust-deep-agents SDK
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.0

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.0"
    exit 1
fi

VERSION=$1

# Validate version format
if [[ ! $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format x.y.z (e.g., 0.1.0)"
    exit 1
fi

echo "Preparing release v$VERSION..."

# Check if we're on a clean git state
if [[ -n $(git status --porcelain) ]]; then
    echo "Error: Working directory is not clean. Please commit or stash your changes."
    exit 1
fi

# Check if we're on the main/dev branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" && "$CURRENT_BRANCH" != "dev" ]]; then
    echo "Warning: You're not on main or dev branch. Current branch: $CURRENT_BRANCH"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update version in all Cargo.toml files
echo "Updating version in Cargo.toml files..."
sed -i.bak "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" crates/*/Cargo.toml
sed -i.bak "s/, version = \"[^\"]*\"/, version = \"$VERSION\"/" crates/*/Cargo.toml

# Clean up backup files
rm -f crates/*/Cargo.toml.bak

# Run tests to make sure everything works
echo "Running tests..."
cargo test --all

# Run clippy
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
echo "Checking formatting..."
cargo fmt --all -- --check

echo "All checks passed!"

# Commit the version changes
git add crates/*/Cargo.toml
git commit -m "chore: bump version to $VERSION"

# Create and push the tag
git tag "v$VERSION"
git push origin "v$VERSION"

echo ""
echo "✅ Release v$VERSION has been tagged and pushed!"
echo "The GitHub Action will now:"
echo "  1. Run tests and checks"
echo "  2. Publish all crates to crates.io"
echo "  3. Create a GitHub release"
echo ""
echo "You can monitor the progress at:"
echo "https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/actions"
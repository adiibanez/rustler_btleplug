git push --delete origin v0.0.1-alpha
git tag -d v0.0.1-alpha
git add .github/
git add native/
git commit -m 'add aarch apple target'
git tag v0.0.1-alpha
git push --tags
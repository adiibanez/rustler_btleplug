#git push --delete origin v0.0.1-alpha
#git tag -d v0.0.1-alpha
git add mix.*
git add .github/
git add native/
git commit -m 'hex roundtrip'
git tag v0.0.5-alpha
git push --tags
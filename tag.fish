#/usr/bin/env fish
git tag "Release" (yj -- -tj < Cargo.toml | jq -r .package.version)

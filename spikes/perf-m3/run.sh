#!/bin/sh
# Builds the two timing pages, serves them from a local preview server (WebKit refuses ES
# modules on file URLs), builds the WKWebView runner and prints both measurements as JSON.
set -e
cd "$(dirname "$0")"
PORT=4178
npx vite build --config vite.config.ts > /dev/null
clang -fobjc-arc -framework AppKit -framework WebKit perf.m -o dist/perf
npx vite preview --config vite.config.ts --port $PORT --strictPort > /dev/null 2>&1 &
SERVER=$!
trap 'kill $SERVER' EXIT
curl --silent --output /dev/null --retry 20 --retry-connrefused --retry-delay 1 "http://localhost:$PORT/"
# Arguments are pages with optional queries, for example "index.html?words=5000&skip=pause".
for page in ${@:-index.html sidebar.html}; do
  echo "$page"; dist/perf "http://localhost:$PORT/$page"
done

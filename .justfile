default:
    just --list

publish:
    CONNECT=wss://ees.kuviman.com cargo geng build --release --platform web --out-dir dist
    butler push dist kuviman/extremely-extreme-sports:html5

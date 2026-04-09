# Generates a self-signed certificate valid for 14 days, to use for webtransport
# run this from /cert
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -keyout key.pem -out cert.pem -days 14 -nodes -subj "/CN=localhost" -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

FINGERPRINT=$(openssl x509 -in "cert.pem" -noout -sha256 -fingerprint | sed 's/^.*=//' | sed 's/://g')
printf '%s' "$FINGERPRINT" > digest.txt

echo "Wrote new fingerprint $FINGERPRINT to digest.txt"
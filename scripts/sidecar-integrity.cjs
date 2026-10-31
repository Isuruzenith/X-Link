const fs = require('fs');
const crypto = require('crypto');

// SHA-256 digests published with the sing-box v1.13.14 release and on wintun.net.
const ARCHIVE_SHA256 = Object.freeze({
  'sing-box-1.13.14-windows-amd64.zip': 'f580782c6dd10f7691c66cea1d7c421813c5fbf7e305d1ee7ce0c3a40d196341',
  'sing-box-1.13.14-darwin-amd64.tar.gz': '5245d645e847f90bb708da74bc020ae078c28489690756419685c04f56b4e3bb',
  'sing-box-1.13.14-darwin-arm64.tar.gz': '73e8967b0fc08e17bce4263ca56ebc394822401a16497a1c4e02316c888202ab',
  'sing-box-1.13.14-linux-amd64.tar.gz': 'f48703461a15476951ac4967cdad339d986f4b8096b4eb3ff0829a500502d697',
  'sing-box-1.13.14-linux-arm64.tar.gz': '4742df6a4314e8ecc41736849fca6d73b8f9e91b6e8b06ee794ff17ba180579e',
  'wintun-0.14.1.zip': '07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51',
});

function assertArchiveDigest(name, actual) {
  const expected = Object.hasOwn(ARCHIVE_SHA256, name) ? ARCHIVE_SHA256[name] : null;
  if (!expected || !/^[a-f0-9]{64}$/.test(actual) || actual !== expected) {
    throw new Error(`Integrity check failed for ${name}`);
  }
}

async function verifyArchive(filePath, archiveName) {
  const hash = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(filePath)) {
    hash.update(chunk);
  }
  assertArchiveDigest(archiveName, hash.digest('hex'));
}

module.exports = { ARCHIVE_SHA256, assertArchiveDigest, verifyArchive };

const fs = require('fs');
const https = require('https');
const path = require('path');
const { execSync } = require('child_process');

const SINGBOX_VERSION = '1.11.0';
const WINTUN_VERSION = '0.14.1';

const binariesDir = path.join(__dirname, '..', 'src-tauri', 'binaries');
fs.mkdirSync(binariesDir, { recursive: true });

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    https.get(url, (response) => {
      if (response.statusCode === 301 || response.statusCode === 302) {
        // Follow redirect
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download ${url}: status code ${response.statusCode}`));
        return;
      }
      const file = fs.createWriteStream(dest);
      response.pipe(file);
      file.on('finish', () => {
        file.close(() => resolve());
      });
    }).on('error', (err) => {
      fs.unlink(dest, () => {});
      reject(err);
    });
  });
}

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'win32') {
    if (arch === 'x64') {
      return {
        triple: 'x86_64-pc-windows-msvc',
        archiveName: `sing-box-${SINGBOX_VERSION}-windows-amd64.zip`,
        binName: 'sing-box.exe',
        isWindows: true,
      };
    }
  } else if (platform === 'darwin') {
    if (arch === 'x64') {
      return {
        triple: 'x86_64-apple-darwin',
        archiveName: `sing-box-${SINGBOX_VERSION}-darwin-amd64.tar.gz`,
        binName: 'sing-box',
        isWindows: false,
      };
    } else if (arch === 'arm64') {
      return {
        triple: 'aarch64-apple-darwin',
        archiveName: `sing-box-${SINGBOX_VERSION}-darwin-arm64.tar.gz`,
        binName: 'sing-box',
        isWindows: false,
      };
    }
  } else if (platform === 'linux') {
    if (arch === 'x64') {
      return {
        triple: 'x86_64-unknown-linux-gnu',
        archiveName: `sing-box-${SINGBOX_VERSION}-linux-amd64.tar.gz`,
        binName: 'sing-box',
        isWindows: false,
      };
    } else if (arch === 'arm64') {
      return {
        triple: 'aarch64-unknown-linux-gnu',
        archiveName: `sing-box-${SINGBOX_VERSION}-linux-arm64.tar.gz`,
        binName: 'sing-box',
        isWindows: false,
      };
    }
  }

  throw new Error(`Unsupported platform/architecture: ${platform}/${arch}`);
}

function findFileRecursively(dir, fileName) {
  const files = fs.readdirSync(dir);
  for (const file of files) {
    const filePath = path.join(dir, file);
    const stat = fs.statSync(filePath);
    if (stat.isDirectory()) {
      const found = findFileRecursively(filePath, fileName);
      if (found) return found;
    } else if (file === fileName) {
      return filePath;
    }
  }
  return null;
}

async function setup() {
  const info = getPlatformInfo();
  const targetName = `sing-box-${info.triple}${info.isWindows ? '.exe' : ''}`;
  const finalDest = path.join(binariesDir, targetName);

  if (fs.existsSync(finalDest)) {
    console.log(`[Sidecar Setup] ${targetName} already exists. Skipping download.`);
  } else {
    console.log(`[Sidecar Setup] Downloading sing-box version ${SINGBOX_VERSION} for ${info.triple}...`);
    const downloadUrl = `https://github.com/SagerNet/sing-box/releases/download/v${SINGBOX_VERSION}/${info.archiveName}`;
    const tempArchive = path.join(binariesDir, `temp_archive${path.extname(info.archiveName)}`);
    const tempExtractDir = path.join(binariesDir, 'temp_extract');

    try {
      await downloadFile(downloadUrl, tempArchive);
      console.log(`[Sidecar Setup] Extracting archive...`);
      fs.mkdirSync(tempExtractDir, { recursive: true });

      if (info.isWindows) {
        execSync(`powershell -Command "Expand-Archive -Path '${tempArchive}' -DestinationPath '${tempExtractDir}' -Force"`);
      } else {
        execSync(`tar -xzf "${tempArchive}" -C "${tempExtractDir}"`);
      }

      const extractedBin = findFileRecursively(tempExtractDir, info.binName);
      if (!extractedBin) {
        throw new Error(`Could not find ${info.binName} inside extracted archive.`);
      }

      fs.renameSync(extractedBin, finalDest);
      console.log(`[Sidecar Setup] Successfully placed sidecar: ${targetName}`);
    } catch (err) {
      console.error(`[Sidecar Setup] Error downloading/extracting sing-box:`, err);
      process.exit(1);
    } finally {
      // Clean up temp
      if (fs.existsSync(tempArchive)) fs.unlinkSync(tempArchive);
      if (fs.existsSync(tempExtractDir)) fs.rmSync(tempExtractDir, { recursive: true, force: true });
    }
  }

  // Windows-specific: wintun.dll setup
  if (info.isWindows) {
    const wintunDest = path.join(binariesDir, 'wintun.dll');
    if (fs.existsSync(wintunDest)) {
      console.log(`[Sidecar Setup] wintun.dll already exists. Skipping.`);
    } else {
      console.log(`[Sidecar Setup] Downloading wintun.dll version ${WINTUN_VERSION}...`);
      const wintunUrl = `https://www.wintun.net/builds/wintun-${WINTUN_VERSION}.zip`;
      const tempArchive = path.join(binariesDir, 'temp_wintun.zip');
      const tempExtractDir = path.join(binariesDir, 'temp_wintun_extract');

      try {
        await downloadFile(wintunUrl, tempArchive);
        console.log(`[Sidecar Setup] Extracting wintun archive...`);
        fs.mkdirSync(tempExtractDir, { recursive: true });

        execSync(`powershell -Command "Expand-Archive -Path '${tempArchive}' -DestinationPath '${tempExtractDir}' -Force"`);

        const extractedDll = path.join(tempExtractDir, 'wintun', 'bin', 'amd64', 'wintun.dll');
        if (!fs.existsSync(extractedDll)) {
          throw new Error('Could not find amd64/wintun.dll inside extracted zip.');
        }

        fs.renameSync(extractedDll, wintunDest);
        console.log(`[Sidecar Setup] Successfully placed wintun.dll`);
      } catch (err) {
        console.error(`[Sidecar Setup] Error downloading/extracting wintun:`, err);
        process.exit(1);
      } finally {
        if (fs.existsSync(tempArchive)) fs.unlinkSync(tempArchive);
        if (fs.existsSync(tempExtractDir)) fs.rmSync(tempExtractDir, { recursive: true, force: true });
      }
    }
  }
}

setup().then(() => console.log('[Sidecar Setup] Setup complete!'));

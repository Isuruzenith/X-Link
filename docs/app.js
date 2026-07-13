document.addEventListener('DOMContentLoaded', () => {
  // 1. Target elements
  const versionBadge = document.getElementById('version-badge');
  const osText = document.getElementById('detect-os-text');
  const btnWin = document.getElementById('btn-win');
  const btnMac = document.getElementById('btn-mac');
  const btnLinux = document.getElementById('btn-linux');

  // Default Fallbacks
  const REPO_RELEASES = 'https://github.com/Isuruzenith/X-Link/releases';
  const API_URL = 'https://api.github.com/repos/Isuruzenith/X-Link/releases/latest';

  // 2. Platform detection
  const getPlatform = () => {
    const userAgent = window.navigator.userAgent.toLowerCase();
    const platform = window.navigator.platform.toLowerCase();

    if (platform.includes('win') || userAgent.includes('windows')) {
      return 'windows';
    }
    if (platform.includes('mac') || userAgent.includes('intel mac') || userAgent.includes('macintosh')) {
      return 'macos';
    }
    if (platform.includes('linux') || userAgent.includes('linux')) {
      return 'linux';
    }
    return 'unknown';
  };

  const detectedOS = getPlatform();

  // Highlight button and set helper text based on OS
  if (detectedOS === 'windows') {
    btnWin.classList.add('highlight');
    osText.innerHTML = '👉 We recommended downloading the <strong>Windows (.msi)</strong> installer for your PC.';
  } else if (detectedOS === 'macos') {
    btnMac.classList.add('highlight');
    osText.innerHTML = '👉 We recommended downloading the <strong>macOS (.dmg)</strong> installer for your Mac.';
  } else if (detectedOS === 'linux') {
    btnLinux.classList.add('highlight');
    osText.innerHTML = '👉 We recommended downloading the <strong>Linux (.deb)</strong> package for your system.';
  } else {
    osText.innerText = 'Select the appropriate installer for your system below:';
  }

  // 3. Fetch latest release assets from GitHub API
  fetch(API_URL)
    .then(response => {
      if (!response.ok) throw new Error('Release API fetch failed');
      return response.json();
    })
    .then(data => {
      if (data && data.tag_name) {
        // Update version badge
        versionBadge.innerText = data.tag_name;

        // Search assets for correct installers
        const assets = data.assets || [];
        
        let winAsset = null;
        let macAsset = null;
        let linuxAsset = null;

        assets.forEach(asset => {
          const name = asset.name.toLowerCase();
          
          // Matches .msi or .exe (prefer .msi)
          if (name.endsWith('.msi')) {
            winAsset = asset.browser_download_url;
          } else if (name.endsWith('.exe') && !winAsset) {
            winAsset = asset.browser_download_url;
          }
          
          // Matches .dmg or .app (prefer .dmg)
          if (name.endsWith('.dmg')) {
            macAsset = asset.browser_download_url;
          }
          
          // Matches .deb or .appimage (prefer .deb)
          if (name.endsWith('.deb')) {
            linuxAsset = asset.browser_download_url;
          } else if (name.endsWith('.appimage') && !linuxAsset) {
            linuxAsset = asset.browser_download_url;
          }
        });

        // Update button links
        if (winAsset) btnWin.href = winAsset;
        if (macAsset) btnMac.href = macAsset;
        if (linuxAsset) btnLinux.href = linuxAsset;
      }
    })
    .catch(err => {
      console.warn('Could not retrieve dynamic release links from GitHub API. Falling back to default release list.', err);
      // Fallback: Buttons remain pointed to the general releases list
      btnWin.href = REPO_RELEASES;
      btnMac.href = REPO_RELEASES;
      btnLinux.href = REPO_RELEASES;
    });
});

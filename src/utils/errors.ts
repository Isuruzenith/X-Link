const ERROR_MAP: Record<string, string> = {
  // Connection lifecycle
  'Failed to initialize sidecar':
    'Could not start the proxy engine. Please restart X-Link.',
  'spawn_failed':
    'The proxy engine failed to launch. Try restarting X-Link or reinstalling.',
  'state_lock_poisoned':
    'An internal error occurred. Please restart X-Link.',

  // Config issues
  'Failed to write default config':
    'Could not save the proxy configuration. Check disk space and permissions.',
  'Failed to deserialize profile nodes':
    'The server profile is corrupted. Please re-import it.',
  'Generated config is invalid JSON':
    'The generated configuration is invalid. This is a bug — please report it.',
  'Configuration check failed':
    'The proxy configuration failed validation. Check your server settings.',
  'No active profile selected':
    'No profile selected. Please import or select a server profile before connecting.',
  'No server profiles found':
    'No servers found. Please import a profile before connecting.',

  // Health check
  'mixed probe failed':
    'Connection health check failed. The proxy server may be unreachable.',
  'TUN probe failed':
    'TUN mode health check failed. Try running X-Link as Administrator.',
  'System proxy fallback spawn failed':
    'System proxy fallback failed to start. Please restart X-Link.',
  'System proxy fallback health check failed':
    'System proxy fallback could not verify connectivity.',
  'Connection failed after':
    'Could not connect after multiple attempts. Check your server settings and network.',

  // Hot reload
  'Hot reload API call failed':
    'Could not update the running proxy. A full reconnect may be needed.',
  'Hot reload rejected by sing-box':
    'The proxy engine rejected the configuration update.',
};

/**
 * Maps a raw Rust backend error string to a user-friendly message.
 * Falls back to the original error if no mapping is found.
 */
export function toUserFriendlyError(rawError: string): string {
  for (const [pattern, friendly] of Object.entries(ERROR_MAP)) {
    if (rawError.includes(pattern)) {
      return friendly;
    }
  }
  return rawError;
}

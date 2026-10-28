// Policy and orchestration for applying Settings / Routing edits while connected.
//
// sing-box is not hot-reloaded: a connected change is applied through a
// validated cold restart (native `reload_active_config` prepares and validates
// the candidate before stopping the owned child). The JS plugin store persists
// edits *before* that restart, so the applier snapshots the previous values and
// restores them (then recovers the connection) when the restart fails.
//
// This module has no store imports so the policy and applier are unit-testable.
import type { Settings } from './store';

export type ApplyClass = 'restart' | 'live' | 'defer';

/** Settings that are purely app-side and never reach the sing-box config. */
const LIVE_KEYS: ReadonlySet<keyof Settings> = new Set([
  'closeToTray', 'theme', 'autostart', 'geoLookupEnabled',
]);

/**
 * Settings that must NOT silently restart a running session. `proxyMode` is the
 * saved *preference*; the running session keeps its effective mode (TUN may have
 * fallen back to system) and the new preference applies on next connect.
 */
const DEFER_KEYS: ReadonlySet<keyof Settings> = new Set(['proxyMode']);

export function classifySetting(key: keyof Settings): ApplyClass {
  if (LIVE_KEYS.has(key)) return 'live';
  if (DEFER_KEYS.has(key)) return 'defer';
  return 'restart';
}

/** Classification of a whole update: restart wins, then defer, then live. */
export function classifySettingsUpdate(updates: Partial<Settings>): ApplyClass {
  const classes = (Object.keys(updates) as (keyof Settings)[])
    .filter((key) => updates[key] !== undefined)
    .map(classifySetting);
  if (classes.includes('restart')) return 'restart';
  if (classes.includes('defer')) return 'defer';
  return 'live';
}

const PORT_KEYS = ['httpPort', 'socksPort', 'mixedPort', 'apiPort'] as const;

/** Returns a user-facing problem with the *current* settings, or null if applicable. */
export function validateSettingsForApply(settings: Settings): string | null {
  for (const key of PORT_KEYS) {
    const port = settings[key];
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      return `${key} must be between 1 and 65535`;
    }
  }
  if (!Number.isInteger(settings.tunMtu) || settings.tunMtu < 576 || settings.tunMtu > 9000) {
    return 'tunMtu must be between 576 and 9000';
  }
  return null;
}

// ── Applier ──────────────────────────────────────────────────────────────────

export type ApplyOutcome =
  | 'applied'         // restart succeeded and the connection is running
  | 'invalid'         // current values are not applicable yet; kept pending
  | 'kept-running'    // restart failed, previous values restored, old session still up
  | 'recovered'       // restart failed, previous values restored, connection recovered
  | 'recovery-failed' // restart failed and recovery failed; connection is down
  | 'skipped';        // not connected anymore; edits apply on next connect

export interface ApplierDeps<S> {
  debounceMs: number;
  isConnected: () => boolean;
  /** Reads whether the proxy is actually running right now (native truth). */
  isRunning: () => Promise<boolean>;
  /** Merge a newly captured baseline into the pending one (first value wins). */
  merge: (existing: S, incoming: S) => S;
  validate: () => string | null;
  /** Cold restart through the native validated reload. */
  reload: () => Promise<void>;
  /** Persist and publish the baseline snapshot again. */
  restore: (baseline: S) => Promise<void>;
  report: (outcome: ApplyOutcome, detail?: string) => void;
}

export function createApplier<S>(deps: ApplierDeps<S>) {
  let pending: S | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let current: Promise<ApplyOutcome | null> = Promise.resolve(null);

  const arm = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { void flush(); }, deps.debounceMs);
  };

  /** Capture the pre-change snapshot and (re)start the debounce window. */
  const schedule = (baseline: S) => {
    pending = pending === null ? baseline : deps.merge(pending, baseline);
    arm();
  };

  const attempt = async (baseline: S): Promise<ApplyOutcome> => {
    try {
      await deps.reload();
      if (await deps.isRunning()) return 'applied';
      throw new Error('proxy is not running after reload');
    } catch (error) {
      const reason = String(error);
      await deps.restore(baseline);
      let stillRunning = false;
      try { stillRunning = await deps.isRunning(); } catch { /* treat as down */ }
      if (stillRunning) { deps.report('kept-running', reason); return 'kept-running'; }
      try {
        await deps.reload();
        if (await deps.isRunning()) { deps.report('recovered', reason); return 'recovered'; }
        deps.report('recovery-failed', reason);
        return 'recovery-failed';
      } catch (recoveryError) {
        deps.report('recovery-failed', `${reason}; recovery: ${recoveryError}`);
        return 'recovery-failed';
      }
    }
  };

  const run = async (): Promise<ApplyOutcome | null> => {
    timer = null;
    if (pending === null) return null;
    if (!deps.isConnected()) {
      pending = null;
      deps.report('skipped');
      return 'skipped';
    }
    const problem = deps.validate();
    if (problem) {
      deps.report('invalid', problem); // keep pending; the next edit re-arms
      return 'invalid';
    }
    const baseline = pending;
    pending = null;
    const outcome = await attempt(baseline);
    if (outcome === 'applied') {
      deps.report('applied');
      if (pending !== null) arm(); // edits made while restarting
    } else {
      // Edits queued on top of a rejected candidate are stale; drop them.
      pending = null;
    }
    return outcome;
  };

  const flush = (): Promise<ApplyOutcome | null> => {
    if (timer) { clearTimeout(timer); timer = null; }
    current = current.then(run, run);
    return current;
  };

  return {
    schedule,
    flush,
    hasPending: () => pending !== null,
  };
}

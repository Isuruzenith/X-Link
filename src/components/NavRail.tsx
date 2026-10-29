import { useEffect, useRef, useState } from 'react';
import { Icon } from './ui/icon';
import xLinkLogo from '../assets/X-Link-logo.png';
import { useConnectionStore } from '../stores/connectionStore';

export type TabId = 'dashboard' | 'profiles' | 'routing' | 'logs' | 'connections' | 'settings';

interface NavRailProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const NAV_ITEMS = [
  { id: 'dashboard', icon: 'speed', label: 'Dashboard' },
  { id: 'profiles', icon: 'folder_open', label: 'Profiles' },
  { id: 'routing', icon: 'alt_route', label: 'Routing' },
  { id: 'connections', icon: 'hub', label: 'Connections' },
  { id: 'logs', icon: 'terminal', label: 'Logs' },
  { id: 'settings', icon: 'settings', label: 'Settings' },
] as const;

export function NavRail({ activeTab, onTabChange }: NavRailProps) {
  const connectionStatus = useConnectionStore((state) => state.connectionStatus);
  const [open, setOpen] = useState(false);
  const [compact, setCompact] = useState(() => window.matchMedia('(max-width: 700px)').matches);
  const menuRef = useRef<HTMLButtonElement>(null);
  const navRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const query = window.matchMedia('(max-width: 700px)');
    const onResize = () => { setCompact(query.matches); setOpen(false); };
    query.addEventListener('change', onResize);
    return () => query.removeEventListener('change', onResize);
  }, []);

  useEffect(() => {
    if (!open || !compact) return;
    const items = navRef.current?.querySelectorAll<HTMLButtonElement>('button');
    const first = items?.[0];
    const last = items?.[items.length - 1];
    navRef.current?.querySelector<HTMLButtonElement>('[aria-current="page"]')?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpen(false);
        menuRef.current?.focus();
      } else if (event.key === 'Tab' && first && last) {
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, compact]);

  const navigate = (tab: TabId) => {
    onTabChange(tab);
    setOpen(false);
    if (compact) menuRef.current?.focus();
  };

  return (
    <>
      <button ref={menuRef} className="mobile-menu-button" type="button" aria-label="Open navigation" aria-expanded={open} aria-controls="primary-navigation" onClick={() => setOpen(true)}>
        <Icon name="menu" size={21} />
      </button>
      {open && compact && <button className="nav-scrim" type="button" aria-label="Close navigation" tabIndex={-1} onClick={() => { setOpen(false); menuRef.current?.focus(); }} />}
      <aside ref={navRef} id="primary-navigation" className={`app-navigation ${open ? 'is-open' : ''}`} aria-label="X-Link navigation" inert={compact && !open}>
        <div className="nav-brand">
          <img src={xLinkLogo} alt="" className="nav-brand-icon" />
          <div><strong>X-Link</strong><span>Network control</span></div>
          <button type="button" className="nav-close" aria-label="Close navigation" onClick={() => { setOpen(false); menuRef.current?.focus(); }}><Icon name="close" size={18} /></button>
        </div>
        <nav className="nav-destinations" aria-label="Main navigation">
          {NAV_ITEMS.map(({ id, icon, label }) => (
            <button key={id} type="button" aria-current={activeTab === id ? 'page' : undefined} className={`nav-destination ${activeTab === id ? 'is-active' : ''}`} onClick={() => navigate(id)}>
              <Icon name={icon} size={19} aria-hidden="true" /><span>{label}</span>
            </button>
          ))}
        </nav>
        <div className="nav-session" role="status">
          <span className={`session-indicator ${connectionStatus}`} aria-hidden="true" />
          <div><strong>{connectionStatus === 'connecting' ? 'Connecting' : connectionStatus === 'connected' ? 'Connected' : 'Disconnected'}</strong><span>Proxy session</span></div>
        </div>
      </aside>
    </>
  );
}

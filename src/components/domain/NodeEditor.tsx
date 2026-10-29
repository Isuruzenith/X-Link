import React from 'react';
import { Icon } from '@/components/ui/icon';
import { useNodeEditorStore } from '../../stores/nodeEditorStore';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { FormInput, FormSelect } from '@/components/form';
import { Switch } from '@/components/ui/switch';

const PROTOCOL_LABELS: Record<string, string> = {
  vless: 'VLESS',
  vmess: 'VMess',
  trojan: 'Trojan',
  shadowsocks: 'Shadowsocks',
  hysteria2: 'Hysteria2',
  tuic: 'TUIC v5',
  wireguard: 'WireGuard',
  ssh: 'SSH',
  socks: 'SOCKS5',
  http: 'HTTP',
};

const Inp = ({ value, onChange, placeholder, type = 'text', mono = false, label }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  mono?: boolean;
  label?: string;
}) => (
  <FormInput type={type} mono={mono} label={label} value={value} onChange={onChange} placeholder={placeholder} />
);

const Sel = ({ value, onChange, options, label }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
  label?: string;
}) => (
  <FormSelect label={label} value={value} onChange={onChange} options={options} />
);

const Toggle = ({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) => (
  <Switch
    checked={checked}
    onCheckedChange={onChange}
    aria-label={label}
    className="shrink-0"
  />
);
 
const ToggleInput = ({ value, onChange, placeholder, mono = false, label }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  mono?: boolean;
  label?: string;
}) => {
  const [show, setShow] = React.useState(false);
  return (
    <div className="node-secret-field">
      <FormInput
        type={show ? 'text' : 'password'}
        mono={mono}
        label={label}
        value={value}
        onChange={onChange}
        placeholder={placeholder}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="node-secret-toggle"
        onClick={() => setShow(!show)}
        aria-label={`${show ? 'Hide' : 'Show'} ${label || placeholder || 'value'}`}
        aria-pressed={show}
      >
        {show ? <Icon name="visibility_off" size={14} /> : <Icon name="visibility" size={14} />}
      </Button>
    </div>
  );
};

const SubCard = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <Card className="node-editor-section p-4 bg-muted/20 flex flex-col gap-3">
    <CardHeader className="p-0 pb-1">
      <CardTitle className="text-sm font-semibold text-foreground">{title}</CardTitle>
    </CardHeader>
    <CardContent className="p-0 flex flex-col gap-3">
      {children}
    </CardContent>
  </Card>
);

export function NodeEditor() {
  const store = useNodeEditorStore();
  const { isOpen, saveError, closeEditor } = store;
  const errorRef = React.useRef<HTMLDivElement>(null);
  const drawerRef = React.useRef<HTMLDivElement>(null);
  const wasOpen = React.useRef(false);
  const returnFocusRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    if (isOpen && !wasOpen.current) {
      returnFocusRef.current = document.activeElement as HTMLElement | null;
      wasOpen.current = true;
      const frame = requestAnimationFrame(() => drawerRef.current?.querySelector<HTMLElement>('[aria-label="Close node editor"]')?.focus());
      return () => cancelAnimationFrame(frame);
    }
    if (!isOpen && wasOpen.current) {
      returnFocusRef.current?.focus();
      wasOpen.current = false;
    }
  }, [isOpen]);

  React.useEffect(() => {
    if (!isOpen) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        closeEditor();
        return;
      }
      if (event.key !== 'Tab') return;
      if (event.target instanceof Element && event.target.closest('md-outlined-select, md-select-option')) return;
      const focusable = Array.from(drawerRef.current?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), md-text-button:not([disabled]), md-filled-button:not([disabled]), md-outlined-button:not([disabled]), md-icon-button:not([disabled]), md-outlined-text-field:not([disabled]), md-outlined-select:not([disabled]), md-switch:not([disabled])') ?? [])
        .filter((element) => element.getClientRects().length > 0);
      if (!focusable.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [isOpen, closeEditor]);

  React.useEffect(() => {
    if (saveError && errorRef.current) {
      errorRef.current.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, [saveError]);

  if (!isOpen) return null;

  const isSpecial = ['socks', 'http', 'wireguard', 'ssh'].includes(store.editProtocol);
  const noTransport = ['socks', 'http', 'hysteria2', 'tuic', 'wireguard', 'ssh'].includes(store.editProtocol);

  return (
    <>
      <div className="node-editor-scrim" onClick={store.closeEditor} aria-hidden="true" />
      <div ref={drawerRef} role="dialog" aria-modal="true" aria-labelledby="node-editor-title" className="node-editor-drawer">
        {/* Header */}
        <div className="node-editor-header">
          <div className="flex items-center gap-2 min-w-0">
            <h2 id="node-editor-title" className="text-lg font-semibold text-foreground">Edit server</h2>
            <Badge variant="outline" className="h-5 px-2 text-[9.5px] font-mono border-border bg-muted/40 text-foreground">
              {PROTOCOL_LABELS[store.editProtocol] || store.editProtocol}
            </Badge>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground hover:text-foreground rounded"
            onClick={store.closeEditor}
            aria-label="Close node editor"
          >
            <Icon name="close" size={16} />
          </Button>
        </div>

        {/* Section tabs */}
        {!isSpecial && (
          <div className="node-editor-tabs">
            {(['basic', 'transport', 'tls'] as const).map((s) => (
              <Button
                key={s}
                variant="ghost"
                onClick={() => store.setSection(s)}
                aria-pressed={store.section === s}
                className={`node-editor-tab ${store.section === s ? 'is-active' : ''}`}
              >
                {s === 'tls' ? 'TLS / Security' : s.charAt(0).toUpperCase() + s.slice(1)}
              </Button>
            ))}
          </div>
        )}

        {/* Body */}
        <div className="node-editor-body">
          {/* BASIC */}
          {(store.section === 'basic' || isSpecial) && (
            <>
              <SubCard title="Common">
                <div className="flex flex-col gap-1.5">
                  <Inp label="Node name (tag)" value={store.editTag} onChange={(v) => store.setField('editTag', v)} placeholder="my-proxy-sg" />
                </div>
                <div className="flex flex-col gap-1.5">
                  <Sel
                    label="Protocol type"
                    value={store.editProtocol}
                    onChange={(v) => {
                      store.setField('editProtocol', v);
                      store.setSection('basic');
                    }}
                    options={[
                      { value: 'vless', label: 'VLESS' },
                      { value: 'vmess', label: 'VMess' },
                      { value: 'trojan', label: 'Trojan' },
                      { value: 'shadowsocks', label: 'Shadowsocks' },
                      { value: 'hysteria2', label: 'Hysteria2' },
                      { value: 'tuic', label: 'TUIC v5' },
                      { value: 'wireguard', label: 'WireGuard' },
                      { value: 'ssh', label: 'SSH' },
                      { value: 'socks', label: 'SOCKS5' },
                      { value: 'http', label: 'HTTP Proxy' },
                    ]}
                  />
                </div>
                <div className="grid grid-cols-3 gap-3">
                  <div className="flex flex-col gap-1.5 col-span-2">
                    <Inp label="Server address" value={store.editAddress} onChange={(v) => store.setField('editAddress', v)} placeholder="sg.example.com" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp label="Port" type="number" value={String(store.editPort)} onChange={(v) => store.setField('editPort', Number(v) || 443)} mono />
                  </div>
                </div>
              </SubCard>

              {(store.editProtocol === 'vless' || store.editProtocol === 'vmess') && (
                <SubCard title="Authentication">
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput label="UUID" value={store.editUuid} onChange={(v) => store.setField('editUuid', v)} placeholder="uuid-here" mono />
                  </div>
                  {store.editProtocol === 'vless' && (
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="Flow (XTLS)"
                        value={store.editFlow}
                        onChange={(v) => store.setField('editFlow', v)}
                        options={[
                          { value: '', label: 'None' },
                          { value: 'xtls-rprx-vision', label: 'xtls-rprx-vision' },
                        ]}
                      />
                    </div>
                  )}
                </SubCard>
              )}

              {store.editProtocol === 'trojan' && (
                <SubCard title="Trojan Authentication">
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Trojan password"
                      value={store.editPassword}
                      onChange={(v) => store.setField('editPassword', v)}
                      placeholder="Trojan password"
                    />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'shadowsocks' && (
                <SubCard title="Shadowsocks">
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Shadowsocks password"
                      value={store.editPassword}
                      onChange={(v) => store.setField('editPassword', v)}
                      placeholder="Password"
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Sel
                      label="Encryption method"
                      value={store.editMethod}
                      onChange={(v) => store.setField('editMethod', v)}
                      options={[
                        { value: 'aes-256-gcm', label: 'AES-256-GCM' },
                        { value: 'aes-128-gcm', label: 'AES-128-GCM' },
                        { value: 'chacha20-ietf-poly1305', label: 'ChaCha20-Poly1305' },
                        { value: '2022-blake3-aes-256-gcm', label: '2022-Blake3-AES-256-GCM' },
                      ]}
                    />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'hysteria2' && (
                <SubCard title="Hysteria2">
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Authentication password"
                      value={store.editHy2Auth}
                      onChange={(v) => store.setField('editHy2Auth', v)}
                      placeholder="Auth Password"
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Upload bandwidth" value={store.editHy2UpBw} onChange={(v) => store.setField('editHy2UpBw', v)} placeholder="100 mbps" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Download bandwidth" value={store.editHy2DownBw} onChange={(v) => store.setField('editHy2DownBw', v)} placeholder="200 mbps" mono />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp label="SNI" value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="your-server.com" />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="Obfuscation"
                        value={store.editHy2ObfsType}
                        onChange={(v) => store.setField('editHy2ObfsType', v)}
                        options={[
                          { value: '', label: 'None' },
                          { value: 'salamander', label: 'Salamander' },
                        ]}
                      />
                    </div>
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'tuic' && (
                <SubCard title="TUIC v5">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <ToggleInput label="TUIC UUID" value={store.editTuicUuid} onChange={(v) => store.setField('editTuicUuid', v)} placeholder="UUID" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <ToggleInput
                        label="TUIC password"
                        value={store.editTuicPassword}
                        onChange={(v) => store.setField('editTuicPassword', v)}
                        placeholder="Password"
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="Congestion"
                        value={store.editTuicCongestion}
                        onChange={(v) => store.setField('editTuicCongestion', v)}
                        options={[
                          { value: 'bbr', label: 'BBR' },
                          { value: 'cubic', label: 'CUBIC' },
                          { value: 'new_reno', label: 'New Reno' },
                        ]}
                      />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="UDP relay mode"
                        value={store.editTuicUdpMode}
                        onChange={(v) => store.setField('editTuicUdpMode', v)}
                        options={[
                          { value: 'native', label: 'Native' },
                          { value: 'quic', label: 'QUIC' },
                        ]}
                      />
                    </div>
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'socks' && (
                <SubCard title="SOCKS">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="SOCKS version"
                        value={String(store.editSocksVersion)}
                        onChange={(v) => store.setField('editSocksVersion', parseInt(v))}
                        options={[
                          { value: '5', label: 'SOCKS5' },
                          { value: '4', label: 'SOCKS4' },
                        ]}
                      />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Username" value={store.editSocksUser} onChange={(v) => store.setField('editSocksUser', v)} placeholder="Optional" />
                    </div>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="SOCKS password"
                      value={store.editSocksPassword}
                      onChange={(v) => store.setField('editSocksPassword', v)}
                      placeholder="Optional"
                    />
                  </div>
                  <div className="flex items-center justify-between p-2 px-3 bg-muted/40 border border-border/40 rounded-lg">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[11.5px] font-bold text-foreground">UDP over TCP</span>
                    </div>
                    <Toggle label="UDP over TCP" checked={store.editSocksUdpOverTcp} onChange={(v) => store.setField('editSocksUdpOverTcp', v)} />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'http' && (
                <SubCard title="HTTP Proxy">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Username" value={store.editHttpUser} onChange={(v) => store.setField('editHttpUser', v)} placeholder="Optional" />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <ToggleInput
                        label="HTTP password"
                        value={store.editHttpPassword}
                        onChange={(v) => store.setField('editHttpPassword', v)}
                        placeholder="Optional"
                      />
                    </div>
                  </div>
                  <div className="flex items-center justify-between p-2 px-3 bg-muted/40 border border-border/40 rounded-lg">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[11.5px] font-bold text-foreground">HTTPS (TLS)</span>
                    </div>
                    <Toggle label="HTTPS (TLS)" checked={store.editHttpTls} onChange={(v) => store.setField('editHttpTls', v)} />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'wireguard' && (
                <SubCard title="WireGuard Interface & Peer">
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Private key (Interface)"
                      value={store.editWgPrivateKey}
                      onChange={(v) => store.setField('editWgPrivateKey', v)}
                      placeholder="Base64 private key"
                      mono
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp
                      label="Local interface address"
                      value={store.editWgAddress}
                      onChange={(v) => store.setField('editWgAddress', v)}
                      placeholder="10.0.0.2/32, fd00::2/128"
                      mono
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp
                      label="Peer public key"
                      value={store.editWgPeerPublicKey}
                      onChange={(v) => store.setField('editWgPeerPublicKey', v)}
                      placeholder="Base64 public key"
                      mono
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Pre-shared key (Optional)"
                      value={store.editWgPsk}
                      onChange={(v) => store.setField('editWgPsk', v)}
                      placeholder="Optional PSK"
                      mono
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp
                        label="MTU"
                        type="number"
                        value={String(store.editWgMtu)}
                        onChange={(v) => store.setField('editWgMtu', Number(v) || 1420)}
                        mono
                      />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Inp
                        label="Reserved bytes (e.g. [0,0,0])"
                        value={store.editWgReserved}
                        onChange={(v) => store.setField('editWgReserved', v)}
                        placeholder="[0,0,0]"
                        mono
                      />
                    </div>
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'ssh' && (
                <SubCard title="SSH Credentials">
                  <div className="flex flex-col gap-1.5">
                    <Inp
                      label="SSH username"
                      value={store.editSshUser}
                      onChange={(v) => store.setField('editSshUser', v)}
                      placeholder="root"
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="SSH password (Optional if using Key)"
                      value={store.editSshPassword}
                      onChange={(v) => store.setField('editSshPassword', v)}
                      placeholder="Password"
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <ToggleInput
                      label="Private key or path (Optional if using Password)"
                      value={store.editSshPrivateKey}
                      onChange={(v) => store.setField('editSshPrivateKey', v)}
                      placeholder="-----BEGIN OPENSSH PRIVATE KEY----- or C:\id_ed25519"
                      mono
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp
                      label="Host key / fingerprint (Optional)"
                      value={store.editSshHostKey}
                      onChange={(v) => store.setField('editSshHostKey', v)}
                      placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5..."
                      mono
                    />
                  </div>
                </SubCard>
              )}
            </>
          )}

          {/* TRANSPORT */}
          {store.section === 'transport' && !noTransport && (
            <SubCard title="Transport Layer">
              <div className="flex flex-col gap-1.5">
                <Sel
                  label="Network transport"
                  value={store.editNetwork}
                  onChange={(v) => store.setField('editNetwork', v)}
                  options={[
                    { value: 'tcp', label: 'TCP (Raw)' },
                    { value: 'ws', label: 'WebSocket' },
                    { value: 'grpc', label: 'gRPC' },
                    { value: 'http', label: 'HTTP/2' },
                    { value: 'httpupgrade', label: 'HTTP Upgrade' },
                    { value: 'quic', label: 'QUIC' },
                  ]}
                />
              </div>
              {(store.editNetwork === 'ws' || store.editNetwork === 'http' || store.editNetwork === 'httpupgrade') && (
                <div className="grid grid-cols-2 gap-3">
                  <div className="flex flex-col gap-1.5">
                    <Inp label="Path" value={store.editPath} onChange={(v) => store.setField('editPath', v)} placeholder="/graphql" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp label="Host header" value={store.editHost} onChange={(v) => store.setField('editHost', v)} placeholder="cdn.example.com" mono />
                  </div>
                </div>
              )}
              {store.editNetwork === 'grpc' && (
                <div className="flex flex-col gap-1.5">
                  <Inp label="Service name" value={store.editServiceName} onChange={(v) => store.setField('editServiceName', v)} placeholder="GunService" mono />
                </div>
              )}
            </SubCard>
          )}

          {/* TLS */}
          {store.section === 'tls' && !noTransport && (
            <>
              <SubCard title="TLS Security">
                <div className="flex flex-col gap-1.5">
                  <Sel
                    label="Security mode"
                    value={store.editTlsEnabled ? (store.editRealityEnabled ? 'reality' : 'tls') : 'none'}
                    onChange={(v) => {
                      if (v === 'none') {
                        store.setField('editTlsEnabled', false);
                        store.setField('editRealityEnabled', false);
                      } else if (v === 'tls') {
                        store.setField('editTlsEnabled', true);
                        store.setField('editRealityEnabled', false);
                      } else {
                        store.setField('editTlsEnabled', true);
                        store.setField('editRealityEnabled', true);
                      }
                    }}
                    options={[
                      { value: 'none', label: 'None' },
                      { value: 'tls', label: 'TLS/SSL' },
                      { value: 'reality', label: 'REALITY (XTLS)' },
                    ]}
                  />
                </div>
                {store.editTlsEnabled && (
                  <div className="flex flex-col gap-1.5">
                    <Sel
                      label="uTLS fingerprint"
                      value={store.editFingerprint}
                      onChange={(v) => store.setField('editFingerprint', v)}
                      options={[
                        { value: 'chrome', label: 'Chrome' },
                        { value: 'firefox', label: 'Firefox' },
                        { value: 'edge', label: 'Edge' },
                        { value: 'safari', label: 'Safari' },
                        { value: 'random', label: 'Random' },
                      ]}
                    />
                  </div>
                )}
              </SubCard>
              {store.editTlsEnabled && (
                <SubCard title="TLS Parameters">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp label="SNI" value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Sel
                        label="ALPN"
                        value={store.editAlpn}
                        onChange={(v) => store.setField('editAlpn', v)}
                        options={[
                          { value: 'http/1.1', label: 'http/1.1 (Default HTTP)' },
                          { value: 'h2', label: 'h2 (HTTP/2 Only)' },
                          { value: 'h2, http/1.1', label: 'h2, http/1.1 (HTTP/2 & HTTP/1.1)' },
                          { value: 'h3', label: 'h3 (HTTP/3 Only)' },
                          { value: 'h2, h3, http/1.1', label: 'h2, h3, http/1.1 (All Protocols)' },
                          { value: '', label: 'None / Auto' }
                        ]}
                      />
                    </div>
                  </div>
                  <div className="flex items-center gap-2 mt-2 px-1">
                    <Checkbox
                      id="nodeAllowInsecure"
                      aria-label="Allow Insecure (skip cert verification)"
                      checked={store.editAllowInsecure}
                      onCheckedChange={(checked) => store.setField('editAllowInsecure', checked)}
                    />
                    <label htmlFor="nodeAllowInsecure" className="text-xs font-semibold text-muted-foreground select-none cursor-pointer">
                      Allow Insecure (skip cert verification)
                    </label>
                  </div>
                </SubCard>
              )}
              {store.editTlsEnabled && store.editRealityEnabled && (
                <SubCard title="REALITY Configuration">
                  <div className="bg-muted/40 border border-border/40 p-2.5 rounded-lg flex items-start gap-2 text-[10px] text-muted-foreground leading-normal shrink-0 mb-1">
                    <Icon name="info" size={14} className="mt-0.5 text-foreground shrink-0" />
                    <span>REALITY impersonates a real TLS server to bypass censorship.</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Inp label="Public key (pbk)" value={store.editPublicKey} onChange={(v) => store.setField('editPublicKey', v)} placeholder="Base64 public key..." mono />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Short ID (sid)" value={store.editShortId} onChange={(v) => store.setField('editShortId', v)} placeholder="97c4e5fcb1e8" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <Inp label="Destination SNI" value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                  </div>
                </SubCard>
              )}
            </>
          )}

          {store.saveError && (
            <div ref={errorRef} role="alert" className="bg-destructive/10 text-destructive text-[11px] px-3.5 py-2.5 rounded border border-destructive/20 flex items-center gap-2 mt-2">
              <Icon name="warning" size={14} className="shrink-0" />
              <span>{store.saveError}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="node-editor-footer">
          <Button variant="outline" size="sm" onClick={store.closeEditor} className="font-semibold text-xs px-4">
            Cancel
          </Button>
          <Button variant="default" size="sm" onClick={store.saveEditor} disabled={store.isSaving} className="font-semibold text-xs px-4 min-w-[90px]">
            {store.isSaving ? <><Icon name="refresh" size={12} className="animate-spin mr-1" /> Saving…</> : 'Save Node'}
          </Button>
        </div>
      </div>
    </>
  );
}

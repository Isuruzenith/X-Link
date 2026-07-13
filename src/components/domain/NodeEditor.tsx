import React from 'react';
import { X, RefreshCw, ShieldAlert, Info } from 'lucide-react';
import { useNodeEditorStore } from '../../stores/nodeEditorStore';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { FormInput, FormSelect } from '@/components/form';
import { Switch } from '@/components/ui/switch';

const PROTOCOL_LABELS: Record<string, string> = {
  vless: 'VLESS',
  vmess: 'VMess',
  trojan: 'Trojan',
  shadowsocks: 'Shadowsocks',
  hysteria2: 'Hysteria2',
  tuic: 'TUIC v5',
  socks: 'SOCKS5',
  http: 'HTTP',
};

const Inp = ({ value, onChange, placeholder, type = 'text', mono = false }: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
  mono?: boolean;
}) => (
  <FormInput
    type={type}
    mono={mono}
    className="h-8"
    value={value}
    onChange={onChange}
    placeholder={placeholder}
  />
);

const Sel = ({ value, onChange, options }: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) => (
  <FormSelect
    className="h-8"
    value={value}
    onChange={onChange}
    options={options}
  />
);

const Toggle = ({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) => (
  <Switch
    checked={checked}
    onCheckedChange={onChange}
    className="shrink-0"
  />
);

const SubCard = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <Card className="p-4 bg-muted/20 border-border/60 shadow-none flex flex-col gap-3">
    <CardHeader className="p-0 pb-1">
      <CardTitle className="text-[11px] font-bold text-muted-foreground uppercase tracking-wider">{title}</CardTitle>
    </CardHeader>
    <CardContent className="p-0 flex flex-col gap-3">
      {children}
    </CardContent>
  </Card>
);

export function NodeEditor() {
  const store = useNodeEditorStore();

  if (!store.isOpen) return null;

  const isSpecial = ['socks', 'http'].includes(store.editProtocol);
  const noTransport = ['socks', 'http', 'hysteria2', 'tuic'].includes(store.editProtocol);

  return (
    <>
      <div className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm transition-all duration-200" onClick={store.closeEditor} />
      <div className="fixed inset-y-0 right-0 z-50 h-full w-[420px] bg-card border-l border-border shadow-lg flex flex-col transition-all duration-300 ease-in-out">
        {/* Header */}
        <div className="flex justify-between items-center px-5 py-4 border-b border-border shrink-0">
          <div className="flex items-center gap-2">
            <span className="text-sm font-bold text-foreground">Edit Node</span>
            <Badge variant="outline" className="h-5 px-2 text-[9.5px] font-mono border-border bg-muted/40 text-foreground">
              {PROTOCOL_LABELS[store.editProtocol] || store.editProtocol}
            </Badge>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="size-7 text-muted-foreground hover:text-foreground rounded"
            onClick={store.closeEditor}
          >
            <X className="size-4" />
          </Button>
        </div>

        {/* Section tabs */}
        {!isSpecial && (
          <div className="flex gap-1 px-5 pt-2 border-b border-border bg-muted/10 shrink-0">
            {(['basic', 'transport', 'tls'] as const).map((s) => (
              <Button
                key={s}
                variant="ghost"
                onClick={() => store.setSection(s)}
                className={`h-9 px-3 text-xs font-bold rounded-none border-b-2 bg-transparent hover:bg-transparent ${
                  store.section === s
                    ? 'border-foreground text-foreground'
                    : 'border-transparent text-muted-foreground hover:text-foreground'
                }`}
              >
                {s === 'tls' ? 'TLS / Security' : s.charAt(0).toUpperCase() + s.slice(1)}
              </Button>
            ))}
          </div>
        )}

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5 flex flex-col gap-4">
          {/* BASIC */}
          {(store.section === 'basic' || isSpecial) && (
            <>
              <SubCard title="Common">
                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] font-bold text-muted-foreground uppercase">Node Name (Tag) *</label>
                  <Inp value={store.editTag} onChange={(v) => store.setField('editTag', v)} placeholder="my-proxy-sg" />
                </div>
                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] font-bold text-muted-foreground uppercase">Protocol Type</label>
                  <Sel
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
                      { value: 'socks', label: 'SOCKS5' },
                      { value: 'http', label: 'HTTP Proxy' },
                    ]}
                  />
                </div>
                <div className="grid grid-cols-3 gap-3">
                  <div className="flex flex-col gap-1.5 col-span-2">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Server Address *</label>
                    <Inp value={store.editAddress} onChange={(v) => store.setField('editAddress', v)} placeholder="sg.example.com" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Port *</label>
                    <input
                      type="number"
                      className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                      value={store.editPort}
                      onChange={(e) => store.setField('editPort', Number(e.target.value) || 443)}
                    />
                  </div>
                </div>
              </SubCard>

              {(store.editProtocol === 'vless' || store.editProtocol === 'vmess') && (
                <SubCard title="Authentication">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">UUID *</label>
                    <Inp value={store.editUuid} onChange={(v) => store.setField('editUuid', v)} placeholder="uuid-here" mono />
                  </div>
                  {store.editProtocol === 'vless' && (
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Flow (XTLS)</label>
                      <Sel
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
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Password *</label>
                    <input
                      type="password"
                      className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                      value={store.editPassword}
                      onChange={(e) => store.setField('editPassword', e.target.value)}
                      placeholder="Trojan password"
                    />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'shadowsocks' && (
                <SubCard title="Shadowsocks">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Password *</label>
                    <input
                      type="password"
                      className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                      value={store.editPassword}
                      onChange={(e) => store.setField('editPassword', e.target.value)}
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Encryption Method *</label>
                    <Sel
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
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Auth Password *</label>
                    <input
                      type="password"
                      className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                      value={store.editHy2Auth}
                      onChange={(e) => store.setField('editHy2Auth', e.target.value)}
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Upload BW</label>
                      <Inp value={store.editHy2UpBw} onChange={(v) => store.setField('editHy2UpBw', v)} placeholder="100 mbps" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Download BW</label>
                      <Inp value={store.editHy2DownBw} onChange={(v) => store.setField('editHy2DownBw', v)} placeholder="200 mbps" mono />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="your-server.com" />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Obfuscation</label>
                      <Sel
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
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">UUID *</label>
                      <Inp value={store.editTuicUuid} onChange={(v) => store.setField('editTuicUuid', v)} placeholder="UUID" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Password *</label>
                      <input
                        type="password"
                        className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                        value={store.editTuicPassword}
                        onChange={(e) => store.setField('editTuicPassword', e.target.value)}
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Congestion</label>
                      <Sel
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
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">UDP Relay Mode</label>
                      <Sel
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
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">SOCKS Version</label>
                      <Sel
                        value={String(store.editSocksVersion)}
                        onChange={(v) => store.setField('editSocksVersion', parseInt(v))}
                        options={[
                          { value: '5', label: 'SOCKS5' },
                          { value: '4', label: 'SOCKS4' },
                        ]}
                      />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Username</label>
                      <Inp value={store.editSocksUser} onChange={(v) => store.setField('editSocksUser', v)} placeholder="Optional" />
                    </div>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Password</label>
                    <input
                      type="password"
                      className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                      value={store.editSocksPassword}
                      onChange={(e) => store.setField('editSocksPassword', e.target.value)}
                      placeholder="Optional"
                    />
                  </div>
                  <div className="flex items-center justify-between p-2 px-3 bg-muted/40 border border-border/40 rounded-lg">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[11.5px] font-bold text-foreground">UDP over TCP</span>
                    </div>
                    <Toggle checked={store.editSocksUdpOverTcp} onChange={(v) => store.setField('editSocksUdpOverTcp', v)} />
                  </div>
                </SubCard>
              )}

              {store.editProtocol === 'http' && (
                <SubCard title="HTTP Proxy">
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Username</label>
                      <Inp value={store.editHttpUser} onChange={(v) => store.setField('editHttpUser', v)} placeholder="Optional" />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Password</label>
                      <input
                        type="password"
                        className="field text-xs bg-background border border-border px-2.5 rounded h-8 text-foreground w-full"
                        value={store.editHttpPassword}
                        onChange={(e) => store.setField('editHttpPassword', e.target.value)}
                        placeholder="Optional"
                      />
                    </div>
                  </div>
                  <div className="flex items-center justify-between p-2 px-3 bg-muted/40 border border-border/40 rounded-lg">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-[11.5px] font-bold text-foreground">HTTPS (TLS)</span>
                    </div>
                    <Toggle checked={store.editHttpTls} onChange={(v) => store.setField('editHttpTls', v)} />
                  </div>
                </SubCard>
              )}
            </>
          )}

          {/* TRANSPORT */}
          {store.section === 'transport' && !noTransport && (
            <SubCard title="Transport Layer">
              <div className="flex flex-col gap-1.5">
                <label className="text-[10px] font-bold text-muted-foreground uppercase">Network Transport</label>
                <Sel
                  value={store.editNetwork}
                  onChange={(v) => store.setField('editNetwork', v)}
                  options={[
                    { value: 'tcp', label: 'TCP (Raw)' },
                    { value: 'ws', label: 'WebSocket' },
                    { value: 'grpc', label: 'gRPC' },
                    { value: 'http', label: 'HTTP/2' },
                    { value: 'httpupgrade', label: 'HTTP Upgrade' },
                  ]}
                />
              </div>
              {(store.editNetwork === 'ws' || store.editNetwork === 'http' || store.editNetwork === 'httpupgrade') && (
                <div className="grid grid-cols-2 gap-3">
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Path</label>
                    <Inp value={store.editPath} onChange={(v) => store.setField('editPath', v)} placeholder="/graphql" mono />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Host Header</label>
                    <Inp value={store.editHost} onChange={(v) => store.setField('editHost', v)} placeholder="cdn.example.com" mono />
                  </div>
                </div>
              )}
              {store.editNetwork === 'grpc' && (
                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] font-bold text-muted-foreground uppercase">Service Name</label>
                  <Inp value={store.editServiceName} onChange={(v) => store.setField('editServiceName', v)} placeholder="GunService" mono />
                </div>
              )}
            </SubCard>
          )}

          {/* TLS */}
          {store.section === 'tls' && !noTransport && (
            <>
              <SubCard title="TLS Security">
                <div className="flex flex-col gap-1.5">
                  <label className="text-[10px] font-bold text-muted-foreground uppercase">Security Mode</label>
                  <Sel
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
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">uTLS Fingerprint</label>
                    <Sel
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
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">ALPN</label>
                      <Sel
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
                    <input
                      type="checkbox"
                      id="nodeAllowInsecure"
                      className="rounded border-border text-foreground focus:ring-ring shrink-0 size-3.5"
                      checked={store.editAllowInsecure}
                      onChange={(e) => store.setField('editAllowInsecure', e.target.checked)}
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
                    <Info className="size-3.5 mt-0.5 text-foreground shrink-0" />
                    <span>REALITY impersonates a real TLS server to bypass censorship.</span>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <label className="text-[10px] font-bold text-muted-foreground uppercase">Public Key (pbk) *</label>
                    <Inp value={store.editPublicKey} onChange={(v) => store.setField('editPublicKey', v)} placeholder="Base64 public key..." mono />
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Short ID (sid)</label>
                      <Inp value={store.editShortId} onChange={(v) => store.setField('editShortId', v)} placeholder="97c4e5fcb1e8" mono />
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <label className="text-[10px] font-bold text-muted-foreground uppercase">Destination SNI</label>
                      <Inp value={store.editServerName} onChange={(v) => store.setField('editServerName', v)} placeholder="aka.ms" mono />
                    </div>
                  </div>
                </SubCard>
              )}
            </>
          )}

          {store.saveError && (
            <div className="bg-destructive/10 text-destructive text-[11px] px-3.5 py-2.5 rounded border border-destructive/20 flex items-center gap-2 mt-2">
              <ShieldAlert className="size-3.5 shrink-0" />
              <span>{store.saveError}</span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-5 py-4 border-t border-border shrink-0 bg-muted/10">
          <Button variant="outline" size="sm" onClick={store.closeEditor} className="font-semibold text-xs h-8.5 px-4">
            Cancel
          </Button>
          <Button variant="default" size="sm" onClick={store.saveEditor} disabled={store.isSaving} className="font-semibold text-xs h-8.5 px-4 min-w-[90px]">
            {store.isSaving ? <><RefreshCw className="size-3 animate-spin mr-1" /> Saving…</> : 'Save Node'}
          </Button>
        </div>
      </div>
    </>
  );
}

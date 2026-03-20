import {
  EditorialPage,
  Section,
  P,
  A,
  Code,
  CodeBlock,
  Caption,
  ComparisonTable,
  List,
  Li,
} from './components'

const TOC = [
  { label: 'Quick start', href: '#quick-start' },
  { label: 'File server', href: '#file-server' },
  { label: 'How it works', href: '#how-it-works' },
  { label: 'CLI reference', href: '#cli-reference' },
  { label: 'Self-hosting', href: '#self-hosting' },
  { label: 'Performance', href: '#performance' },
  { label: 'AI skill', href: '#ai-skill' },
  { label: 'Comparison', href: '#comparison' },
  { label: 'Security', href: '#security' },
]

export default function App() {
  return (
    <EditorialPage toc={TOC}>
      <P>
        Expose anything to the internet — local ports, files, directories. One command, instant HTTPS tunnel. Built in <strong>Rust</strong> — single binary, 8 MB memory, zero-copy data plane. <A href="https://github.com/jiweiyuan/tunelo">Star on GitHub</A>.
      </P>

      <CodeBlock lang="bash" showLineNumbers={false}>{`$ tunelo http 3000
  ✔ Tunnel is ready!

  Public URL:  https://abc123.tunelo.net
  Forwarding:  → http://localhost:3000`}</CodeBlock>

      <CodeBlock lang="bash" showLineNumbers={false}>{`$ tunelo .
  ▸ Serving /Users/you/project on :51234
  ✔ Tunnel is ready!

  Public URL:  https://xyz789.tunelo.net
  Forwarding:  → file server (web explorer)`}</CodeBlock>

      <P>
        Other tunnel tools need config files, accounts, or dashboards. Tunelo is a <strong>single binary</strong> that does two things: expose a local port, or serve files with a built-in web explorer. QUIC transport gives you multiplexed, encrypted, low-latency tunneling. By default, the client connects to the public relay at <Code>tunelo.net</Code> — or point it at your own with <Code>--relay</Code>.
      </P>

      <Section id="quick-start" title="Quick start">
        <P>
          <strong>Two terminals</strong> and you're sharing a local service with the world.
        </P>

        <P>1. Start your local service:</P>
        <CodeBlock lang="bash">{`python3 -m http.server 3000`}</CodeBlock>

        <P>2. Expose it:</P>
        <CodeBlock lang="bash">{`tunelo http 3000`}</CodeBlock>

        <P>
          That's it — no flags needed. The client connects to the public relay at <Code>tunelo.net</Code> by default, assigns you a public URL like <Code>https://abc123.tunelo.net</Code>, and starts relaying traffic to your localhost through an encrypted QUIC tunnel.
        </P>

        <P>Install the <strong>skill</strong> to teach your AI coding agent how to use tunelo:</P>
        <CodeBlock lang="bash" showLineNumbers={false}>{`npx -y skills add tunelo/tunelo`}</CodeBlock>

        <Caption>The skill teaches your agent the CLI, when to use tunnels, and handles setup automatically.</Caption>
      </Section>

      <Section id="file-server" title="File server">
        <P>
          Run <Code>tunelo .</Code> or <Code>tunelo ./dist</Code> and tunelo starts a <strong>built-in file server</strong> with a React web explorer — embedded directly in the binary. Browse directories, preview files, share them through a public URL. No nginx, no Python server, no external dependencies.
        </P>

        <CodeBlock lang="bash">{`# Serve current directory through a tunnel
tunelo .

# Serve a specific directory
tunelo ./dist

# Local-only preview (no tunnel)
tunelo . --local

# Local preview on a specific port
tunelo . -l -p 8000

# With custom subdomain
tunelo . --subdomain files`}</CodeBlock>

        <P>The web explorer supports:</P>
        <List>
          <Li><strong>Directory browsing</strong> — navigate folders with breadcrumbs, sorted listing</Li>
          <Li><strong>Code viewer</strong> — syntax highlighting for source files</Li>
          <Li><strong>Markdown</strong> — rendered with full formatting</Li>
          <Li><strong>PDF viewer</strong> — embedded PDF rendering</Li>
          <Li><strong>Media players</strong> — video, audio, image preview with seeking support</Li>
          <Li><strong>Data tables</strong> — CSV and Excel file viewer</Li>
          <Li><strong>Range requests</strong> — streaming for large files, seeking in media</Li>
        </List>

        <P>
          The frontend is compiled into the binary at build time via <Code>include_dir!</Code>. The file server API is two endpoints: <Code>/_api/ls</Code> for directory listings and <Code>/_api/raw</Code> for file content. Everything else serves the SPA.
        </P>

        <Caption>One binary. File server + web explorer + tunnel. No setup.</Caption>
      </Section>

      <Section id="how-it-works" title="How it works">
        <P>
          The client opens a <strong>QUIC connection</strong> to the relay and registers a subdomain. When a browser hits that subdomain, the relay peeks at the Host header, finds the matching tunnel, opens a QUIC stream, and does <Code>copy_bidirectional</Code> between the TCP socket and the QUIC stream. <strong>Zero HTTP parsing</strong> on the data path.
        </P>

        <CodeBlock lang="bash" showLineNumbers={false}>{`Browser → HTTPS → Relay → QUIC stream → Client → localhost:3000
                 (8 MB)                  (8 MB)

┌──────────┐   ┌────────────────┐   ┌───────────────┐   ┌───────────┐
│  Browser  │──▶│  tunelo relay  │◀──│ tunelo client  │──▶│ localhost │
│           │   │  TLS + routing │   │  QUIC tunnel   │   │   :3000   │
└──────────┘   └────────────────┘   └───────────────┘   └───────────┘
  Internet         Your VPS           Your machine         Your app`}</CodeBlock>

        <List>
          <Li><strong>Control stream</strong> — one persistent QUIC stream for registration + heartbeats (msgpack framing)</Li>
          <Li><strong>Data streams</strong> — new bidirectional QUIC stream per HTTP request, zero-copy relay</Li>
          <Li><strong>No buffering</strong> — <Code>tokio::io::copy_bidirectional</Code> between TCP and QUIC, no intermediate copies</Li>
        </List>

        <Caption>Only the control stream uses serialization. Data streams are raw bytes.</Caption>
      </Section>

      <Section id="cli-reference" title="CLI reference">
        <P>
          The client and relay are <strong>fully decoupled</strong>. The client defaults to the public relay at <Code>tunelo.net:4433</Code> — or use <Code>--relay</Code> to point at your own. No account, no signup.
        </P>

        <CodeBlock lang="bash">{`# Port mode — expose a local HTTP service
tunelo http 3000
tunelo http 3000 --subdomain myapp
tunelo http 3000 --relay my.server:4433
tunelo http 3000 -H 192.168.1.100
tunelo http 3000 --private
tunelo http 3000 --code mysecret

# File mode — serve files with web explorer
tunelo .
tunelo ./dist
tunelo . --subdomain files
tunelo . --local
tunelo . -l -p 8000`}</CodeBlock>

        <P>Relay configuration:</P>

        <CodeBlock lang="bash">{`# Start with defaults
tunelo-relay

# Production deployment
tunelo-relay --domain tunelo.net

# Custom addresses
tunelo-relay --tunnel-addr 0.0.0.0:4433 --http-addr 0.0.0.0:80`}</CodeBlock>

        <Caption>Client: one binary, one command. Relay: three flags.</Caption>
      </Section>

      <Section id="self-hosting" title="Self-hosting">
        <P>
          Don't want to use the public relay? Run your own on any VPS. The client and relay are separate binaries — just point <Code>--relay</Code> at your server. The <A href="https://github.com/jiweiyuan/tunelo/tree/main/deploy">deploy/</A> directory has everything: systemd service, nginx config, Let's Encrypt with Cloudflare DNS.
        </P>

        <CodeBlock lang="bash">{`# Build the relay
cargo build --release --bin tunelo-relay

# Run it
./target/release/tunelo-relay --domain yourdomain.com

# Point your clients to it
tunelo http 3000 --relay yourdomain.com:4433`}</CodeBlock>

        <P>You need:</P>
        <List>
          <Li>A domain with wildcard DNS (<Code>*.yourdomain.com</Code> → your VPS IP)</Li>
          <Li>A wildcard TLS certificate (Let's Encrypt + DNS-01 challenge)</Li>
          <Li>Nginx for TLS termination, proxying <Code>*.yourdomain.com:443</Code> → relay <Code>:8080</Code></Li>
          <Li>UDP port 4433 open for QUIC tunnel connections</Li>
        </List>
      </Section>

      <Section id="performance" title="Performance">
        <P>
          Measured on localhost. Zero errors under 5,000 request stress test. The relay is a <strong>data-plane proxy</strong> — it copies bytes, not parses them.
        </P>

        <ComparisonTable
          headers={['Metric', 'Value']}
          rows={[
            ['Relay memory', '8 MB RSS'],
            ['Client memory', '8 MB RSS'],
            ['Binary size (relay)', '3.5 MB (stripped, LTO)'],
            ['Binary size (client)', '3.3 MB (stripped, LTO)'],
            ['Tunnel overhead vs direct', '~14% (0.56s vs 0.49s / 100 req)'],
            ['Sequential latency', '~6ms/req (localhost)'],
            ['Throughput (20 concurrent)', '670 req/s'],
            ['Throughput (200 concurrent)', '672 req/s'],
            ['Errors under stress (5000 req)', '0'],
            ['Source files', '9'],
            ['Lines of Rust', '1,165'],
          ]}
        />

        <Caption>No GC pauses. No runtime overhead. Just async Rust.</Caption>
      </Section>

      <Section id="ai-skill" title="AI skill">
        <P>
          Tunelo ships with a <strong>SKILL.md</strong> that teaches any AI coding agent how to create tunnels. One command to install it:
        </P>

        <CodeBlock lang="bash" showLineNumbers={false}>{`npx -y skills add tunelo/tunelo`}</CodeBlock>

        <P>
          The skill tells your agent what tunelo does, when to use it, and all the CLI flags. After installing, your agent can run <Code>tunelo http 3000</Code> when you ask it to "share this locally" or "expose my dev server."
        </P>

        <P>What the skill teaches:</P>
        <List>
          <Li>Expose HTTP services with <Code>tunelo http &lt;port&gt;</Code></Li>
          <Li>Request custom subdomains with <Code>--subdomain</Code></Li>
          <Li>Forward to non-localhost hosts with <Code>-H</Code></Li>
          <Li>How the QUIC tunnel works</Li>
        </List>

        <Caption>Follows the same skill format as <A href="https://github.com/runbrowser/runbrowser">RunBrowser</A>.</Caption>
      </Section>

      <Section id="comparison" title="Comparison">
        <P>
          Why use tunelo over the alternatives.
        </P>

        <ComparisonTable
          title="vs ngrok"
          headers={['', 'ngrok', 'tunelo']}
          rows={[
            ['Price', 'Free tier limited', 'Free & open source'],
            ['Account required', 'Yes', 'No'],
            ['Self-hostable', 'No', 'Yes'],
            ['Transport', 'HTTP/2', 'QUIC'],
            ['Binary size', '~25 MB', '3.5 MB'],
            ['Memory', '~50 MB', '8 MB'],
          ]}
        />

        <ComparisonTable
          title="vs bore"
          headers={['', 'bore', 'tunelo']}
          rows={[
            ['Routing', 'By TCP port', 'By subdomain (hostname)'],
            ['Transport', 'TCP', 'QUIC (multiplexed)'],
            ['TLS', 'No built-in', 'TLS at relay'],
            ['Public URL', 'bore.pub:PORT', 'subdomain.tunelo.net'],
            ['HTTP-aware', 'No', 'Yes (Host header routing)'],
          ]}
        />

        <ComparisonTable
          title="vs Cloudflare Tunnel"
          headers={['', 'cloudflared', 'tunelo']}
          rows={[
            ['Self-hostable', 'No', 'Yes'],
            ['Vendor lock-in', 'Cloudflare only', 'Any VPS'],
            ['Transport', 'HTTP/2 (h2mux)', 'QUIC'],
            ['Complexity', 'Named tunnels, DNS config', 'One command'],
            ['Open source relay', 'No', 'Yes'],
          ]}
        />
      </Section>

      <Section id="security" title="Security">
        <P>
          The tunnel is <strong>outbound-only</strong> — the client connects to the relay, not the other way around. No inbound ports needed on your machine.
        </P>

        <List>
          <Li><strong>QUIC encryption</strong> — tunnel traffic is encrypted with TLS 1.3 (rustls)</Li>
          <Li><strong>TLS termination</strong> — public HTTPS at the relay with Let's Encrypt certificates</Li>
          <Li><strong>No data storage</strong> — the relay copies bytes, doesn't inspect or store them</Li>
          <Li><strong>Private tunnels</strong> — optional access code protection with cookie-based auth</Li>
          <Li><strong>Self-hostable</strong> — run your own relay, control your own data</Li>
        </List>
      </Section>

      <P>
        MIT License · <A href="https://github.com/jiweiyuan/tunelo">GitHub</A>
      </P>
    </EditorialPage>
  )
}

import {
  Section,
  P,
  A,
  Code,
  CodeBlock,
  Caption,
  ComparisonTable,
  List,
  Li,
} from '../components'

export const TOC_ZH = [
  { label: '安装', href: '#install' },
  { label: '快速开始', href: '#quick-start' },
  { label: '文件服务器', href: '#file-server' },
  { label: '工作原理', href: '#how-it-works' },
  { label: '命令参考', href: '#cli-reference' },
  { label: '自建部署', href: '#self-hosting' },
  { label: '性能', href: '#performance' },
  { label: 'AI 技能', href: '#ai-skill' },
  { label: '对比', href: '#comparison' },
  { label: '安全性', href: '#security' },
]

export function ContentZh() {
  return (
    <>
      <P>
        将任何东西暴露到互联网 —— 本地端口、文件、目录。一条命令，即刻获得 HTTPS 隧道。使用 <strong>Rust</strong> 构建 —— 单一二进制文件，8 MB 内存，零拷贝数据面。<A href="https://github.com/jiweiyuan/tunelo">在 GitHub 上点 Star</A>。
      </P>

      <CodeBlock lang="bash" showLineNumbers={false}>{`$ tunelo port 3000
  Tunnel is ready.

  Public URL:  https://swift-fox-3847.tunelo.net
  Forwarding:  http://localhost:3000`}</CodeBlock>

      <CodeBlock lang="bash" showLineNumbers={false}>{`$ tunelo serve .
  Serving /Users/you/project on :51234
  Tunnel is ready.

  Public URL:  https://calm-river-9012.tunelo.net
  Forwarding:  http://127.0.0.1:51234`}</CodeBlock>

      <P>
        其他隧道工具需要配置文件、账号或管理面板。Tunelo 是一个<strong>单一二进制文件</strong>，只做两件事：暴露本地端口，或通过内置的 Web 文件浏览器提供文件服务。QUIC 传输协议提供多路复用、加密、低延迟的隧道。默认情况下，客户端连接到 <Code>tunelo.net</Code> 公共中继 —— 或使用 <Code>--relay</Code> 指向你自己的中继服务器。
      </P>

      <Section id="install" title="安装">
        <P><strong>macOS / Linux：</strong></P>
        <CodeBlock lang="bash" showLineNumbers={false}>{`curl -fsSL https://tunelo.net/install.sh | sh`}</CodeBlock>

        <P><strong>Windows (PowerShell)：</strong></P>
        <CodeBlock lang="bash" showLineNumbers={false}>{`irm https://tunelo.net/install.ps1 | iex`}</CodeBlock>

        <P>支持的平台：</P>
        <ComparisonTable
          headers={['操作系统', '架构', '二进制文件']}
          rows={[
            ['Linux', 'x86_64 / arm64', 'tunelo-linux-amd64 / arm64'],
            ['macOS', 'x86_64 / arm64', 'tunelo-macos-amd64 / arm64'],
            ['Windows', 'x86_64', 'tunelo-windows-amd64.exe'],
          ]}
        />

        <P>
          也可以直接从 <A href="https://github.com/jiweiyuan/tunelo/releases">GitHub Releases</A> 下载。
        </P>
      </Section>

      <Section id="quick-start" title="快速开始">
        <P>
          <strong>两个终端</strong>，就能与全世界分享你的本地服务。
        </P>

        <P>1. 启动本地服务：</P>
        <CodeBlock lang="bash">{`python3 -m http.server 3000`}</CodeBlock>

        <P>2. 暴露它：</P>
        <CodeBlock lang="bash">{`tunelo port 3000`}</CodeBlock>

        <P>
          就这么简单 —— 不需要任何参数。客户端默认连接到 <Code>tunelo.net</Code> 公共中继，为你分配一个类似 <Code>https://swift-fox-3847.tunelo.net</Code> 的公共 URL，并通过加密的 QUIC 隧道将流量转发到你的 localhost。
        </P>

        <P>或者一步到位，运行命令并创建隧道：</P>
        <CodeBlock lang="bash">{`tunelo port 3000 -- pnpm dev`}</CodeBlock>

        <P>
          Tunelo 会启动 <Code>pnpm dev</Code>，等待端口 3000 就绪，然后创建隧道。任一方停止时，另一方也会自动清理。
        </P>

        <P>安装 <strong>skill</strong> 来教你的 AI 编程助手使用 tunelo：</P>
        <CodeBlock lang="bash" showLineNumbers={false}>{`npx -y skills add tunelo/tunelo`}</CodeBlock>

        <Caption>该 skill 会教你的 AI 助手了解 CLI 用法、何时使用隧道，并自动完成配置。</Caption>
      </Section>

      <Section id="file-server" title="文件服务器">
        <P>
          运行 <Code>tunelo serve .</Code> 或 <Code>tunelo serve ./dist</Code>，tunelo 会启动<strong>内置文件服务器</strong>，配备 React Web 文件浏览器 —— 直接嵌入到二进制文件中。浏览目录、预览文件、通过公共 URL 分享。无需 nginx、无需 Python 服务器、无需外部依赖。
        </P>

        <CodeBlock lang="bash">{`# 通过隧道提供当前目录
tunelo serve .

# 提供指定目录
tunelo serve ./dist

# 提供单个文件
tunelo serve README.md
tunelo serve index.html

# 仅本地预览（不创建隧道）
tunelo serve . --local

# 在指定端口上本地预览
tunelo serve . -l -p 8000`}</CodeBlock>

        <P>Web 文件浏览器支持：</P>
        <List>
          <Li><strong>目录浏览</strong> —— 面包屑导航，排序文件列表</Li>
          <Li><strong>代码查看器</strong> —— 源代码语法高亮</Li>
          <Li><strong>Markdown</strong> —— 完整格式渲染</Li>
          <Li><strong>PDF 查看器</strong> —— 嵌入式 PDF 渲染</Li>
          <Li><strong>媒体播放器</strong> —— 视频、音频、图片预览，支持拖动播放</Li>
          <Li><strong>数据表格</strong> —— CSV 和 Excel 文件查看</Li>
          <Li><strong>范围请求</strong> —— 大文件流式传输，媒体拖动播放</Li>
        </List>

        <P>
          前端在构建时通过 <Code>include_dir!</Code> 编译进二进制文件。一个二进制 —— 客户端、文件服务器和中继合为一体。文件服务器 API 只有两个端点：<Code>/_api/ls</Code> 用于目录列表，<Code>/_api/raw</Code> 用于文件内容。其他请求都由 SPA 处理。
        </P>

        <Caption>一个二进制文件。文件服务器 + Web 浏览器 + 隧道。无需配置。</Caption>
      </Section>

      <Section id="how-it-works" title="工作原理">
        <P>
          客户端向中继发起 <strong>QUIC 连接</strong>并获得一个子域名。当浏览器访问该 URL 时，中继会检查 Host 头部，找到匹配的隧道，打开一个 QUIC 流，在 TCP 套接字和 QUIC 流之间执行 <Code>copy_bidirectional</Code>。数据路径上<strong>零 HTTP 解析</strong>。断线重连时，客户端会自动请求之前的子域名 —— URL 在网络波动后保持不变。
        </P>

        <CodeBlock lang="bash" showLineNumbers={false}>{`浏览器 → HTTPS → 中继 → QUIC 流 → 客户端 → localhost:3000
                 (8 MB)                (8 MB)

┌──────────┐   ┌────────────────┐   ┌───────────────┐   ┌───────────┐
│   浏览器  │──▶│  tunelo relay  │◀──│ tunelo client  │──▶│ localhost │
│           │   │  TLS + 路由    │   │  QUIC 隧道     │   │   :3000   │
└──────────┘   └────────────────┘   └───────────────┘   └───────────┘
    互联网          你的 VPS            你的机器           你的应用`}</CodeBlock>

        <List>
          <Li><strong>控制流</strong> —— 一条持久 QUIC 流，用于注册 + 心跳（msgpack 帧协议）</Li>
          <Li><strong>数据流</strong> —— 每个 HTTP 请求对应一条新的双向 QUIC 流，零拷贝中继</Li>
          <Li><strong>无缓冲</strong> —— 在 TCP 和 QUIC 之间使用 <Code>tokio::io::copy_bidirectional</Code>，无中间拷贝</Li>
        </List>

        <Caption>只有控制流使用序列化。数据流是原始字节。</Caption>
      </Section>

      <Section id="cli-reference" title="命令参考">
        <P>
          客户端和中继<strong>完全解耦</strong>。客户端默认连接 <Code>tunelo.net:4433</Code> 公共中继 —— 或使用 <Code>--relay</Code> 指向你自己的服务器。无需账号，无需注册。
        </P>

        <CodeBlock lang="bash">{`# 端口模式 —— 暴露本地端口
tunelo port 3000
tunelo port 3000 --relay my.server:4433
tunelo port 3000 -H 192.168.1.100
tunelo port 3000 --password
tunelo port 3000 --password mysecret

# 运行命令并创建隧道
tunelo port 3000 -- pnpm dev
tunelo port 3000 -- next start
tunelo port 5173 -- vite

# 文件模式 —— 提供文件或目录
tunelo serve .
tunelo serve ./dist
tunelo serve README.md
tunelo serve . --local
tunelo serve . -l -p 8000`}</CodeBlock>

        <P>中继配置：</P>

        <CodeBlock lang="bash">{`# 使用默认配置启动
tunelo relay

# 生产环境部署
tunelo relay --domain tunelo.net

# 自定义地址
tunelo relay --tunnel-addr 0.0.0.0:4433 --http-addr 0.0.0.0:80`}</CodeBlock>

        <Caption>客户端：一个二进制，一条命令。中继：三个参数。</Caption>
      </Section>

      <Section id="self-hosting" title="自建部署">
        <P>
          不想用公共中继？在任意 VPS 上运行你自己的。客户端和中继是同一个二进制文件 —— 只需用 <Code>--relay</Code> 指向你的服务器。<A href="https://github.com/jiweiyuan/tunelo/tree/main/deploy">deploy/</A> 目录包含一切：systemd 服务、nginx 配置、Let's Encrypt + Cloudflare DNS。
        </P>

        <CodeBlock lang="bash">{`# 构建中继
cargo build --release --bin tunelo

# 运行
./target/release/tunelo relay --domain yourdomain.com

# 将客户端指向它
tunelo port 3000 --relay yourdomain.com:4433`}</CodeBlock>

        <P>你需要：</P>
        <List>
          <Li>一个配置了泛域名 DNS 的域名（<Code>*.yourdomain.com</Code> → 你的 VPS IP）</Li>
          <Li>泛域名 TLS 证书（Let's Encrypt + DNS-01 验证）</Li>
          <Li>Nginx 用于 TLS 终止，将 <Code>*.yourdomain.com:443</Code> 代理到中继的 <Code>:8080</Code></Li>
          <Li>UDP 端口 4433 开放，用于 QUIC 隧道连接</Li>
        </List>
      </Section>

      <Section id="performance" title="性能">
        <P>
          在 localhost 上测量。5,000 个请求压力测试零错误。中继是一个<strong>数据面代理</strong> —— 它只复制字节，不解析内容。
        </P>

        <ComparisonTable
          headers={['指标', '数值']}
          rows={[
            ['中继内存', '8 MB RSS'],
            ['客户端内存', '8 MB RSS'],
            ['二进制大小', '~4 MB（strip + LTO）'],
            ['隧道开销 vs 直连', '~14%（0.56s vs 0.49s / 100 请求）'],
            ['顺序延迟', '~6ms/请求（localhost）'],
            ['吞吐量（20 并发）', '670 请求/秒'],
            ['吞吐量（200 并发）', '672 请求/秒'],
            ['压力测试错误（5000 请求）', '0'],
          ]}
        />

        <Caption>无 GC 暂停。无运行时开销。纯异步 Rust。</Caption>
      </Section>

      <Section id="ai-skill" title="AI 技能">
        <P>
          Tunelo 附带了一个 <strong>SKILL.md</strong>，可以教任何 AI 编程助手如何创建隧道。一条命令即可安装：
        </P>

        <CodeBlock lang="bash" showLineNumbers={false}>{`npx -y skills add tunelo/tunelo`}</CodeBlock>

        <P>
          该 skill 告诉你的 AI 助手 tunelo 的功能、使用场景和所有 CLI 参数。安装后，当你要求"分享这个本地服务"或"暴露我的开发服务器"时，AI 助手就能运行 <Code>tunelo port 3000</Code>。
        </P>

        <P>skill 包含的内容：</P>
        <List>
          <Li>使用 <Code>tunelo port &lt;port&gt;</Code> 暴露端口</Li>
          <Li>使用 <Code>tunelo port 3000 -- pnpm dev</Code> 运行命令并创建隧道</Li>
          <Li>使用 <Code>--password</Code> 进行密码保护</Li>
          <Li>使用 <Code>tunelo serve .</Code> 提供文件服务</Li>
        </List>

        <Caption>遵循与 <A href="https://github.com/runbrowser/runbrowser">RunBrowser</A> 相同的 skill 格式。</Caption>
      </Section>

      <Section id="comparison" title="对比">
        <P>
          为什么选择 tunelo 而不是其他方案。
        </P>

        <ComparisonTable
          title="vs ngrok"
          headers={['', 'ngrok', 'tunelo']}
          rows={[
            ['价格', '免费版有限制', '免费且开源'],
            ['需要账号', '是', '否'],
            ['可自建', '否', '是'],
            ['传输协议', 'HTTP/2', 'QUIC'],
            ['二进制大小', '~25 MB', '~4 MB'],
            ['内存', '~50 MB', '8 MB'],
          ]}
        />

        <ComparisonTable
          title="vs bore"
          headers={['', 'bore', 'tunelo']}
          rows={[
            ['路由方式', '按 TCP 端口', '按子域名（主机名）'],
            ['传输协议', 'TCP', 'QUIC（多路复用）'],
            ['TLS', '无内置', '中继端 TLS'],
            ['公共 URL', 'bore.pub:PORT', 'subdomain.tunelo.net'],
            ['HTTP 感知', '否', '是（Host 头部路由）'],
          ]}
        />

        <ComparisonTable
          title="vs Cloudflare Tunnel"
          headers={['', 'cloudflared', 'tunelo']}
          rows={[
            ['可自建', '否', '是'],
            ['供应商锁定', '仅 Cloudflare', '任意 VPS'],
            ['传输协议', 'HTTP/2 (h2mux)', 'QUIC'],
            ['复杂度', '命名隧道、DNS 配置', '一条命令'],
            ['开源中继', '否', '是'],
          ]}
        />
      </Section>

      <Section id="security" title="安全性">
        <P>
          隧道是<strong>仅出站</strong>的 —— 客户端连接到中继，而不是相反。你的机器不需要开放入站端口。
        </P>

        <List>
          <Li><strong>QUIC 加密</strong> —— 隧道流量使用 TLS 1.3 加密（rustls）</Li>
          <Li><strong>TLS 终止</strong> —— 中继端使用 Let's Encrypt 证书提供公共 HTTPS</Li>
          <Li><strong>不存储数据</strong> —— 中继只复制字节，不检查或存储内容</Li>
          <Li><strong>密码保护</strong> —— 可选的 <Code>--password</Code>，基于 Cookie 认证</Li>
          <Li><strong>可自建</strong> —— 运行自己的中继，掌控自己的数据</Li>
        </List>
      </Section>

      <P>
        MIT 许可证 · <A href="https://github.com/jiweiyuan/tunelo">GitHub</A>
      </P>
    </>
  )
}

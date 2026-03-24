import { useEffect, useRef, useState } from 'react'
import Prism from 'prismjs'
import 'prismjs/components/prism-bash'
import 'prismjs/components/prism-json'
import 'prismjs/components/prism-typescript'
import { useI18n } from './i18n'

/* ===== TOC sidebar ===== */

function useActiveTocId() {
  const [activeId, setActiveId] = useState('')
  useEffect(() => {
    const headings = document.querySelectorAll<HTMLElement>('h1[id]')
    if (!headings.length) return

    const observer = new IntersectionObserver(
      (entries) => {
        const visible: string[] = []
        entries.forEach((e) => { if (e.isIntersecting && e.target.id) visible.push(e.target.id) })
        if (visible.length) {
          const sorted = visible.sort((a, b) => {
            const elA = document.getElementById(a)
            const elB = document.getElementById(b)
            if (!elA || !elB) return 0
            return elA.getBoundingClientRect().top - elB.getBoundingClientRect().top
          })
          setActiveId(sorted[sorted.length - 1])
        }
      },
      { rootMargin: '-80px 0px -75% 0px', threshold: 0 },
    )
    headings.forEach((h) => observer.observe(h))
    return () => observer.disconnect()
  }, [])
  return activeId
}

export function TableOfContents({ items }: { items: Array<{ label: string; href: string }> }) {
  const activeId = useActiveTocId()

  return (
    <aside
      className="fixed top-[110px] hidden lg:block"
      style={{ left: 'max(1rem, calc((100vw - 550px) / 2 - 200px))', width: '122px' }}
    >
      <nav>
        <a
          href="/"
          className="no-underline transition-colors block"
          style={{
            fontSize: '14px', fontWeight: 700, lineHeight: '20px',
            letterSpacing: '-0.09px', padding: '4px 0',
            color: 'var(--text-primary)', fontFamily: 'var(--font-primary)',
            marginBottom: '8px',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--text-hover)' }}
          onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-primary)' }}
        >
          tunelo
        </a>
        {items.map((item) => {
          const isActive = `#${activeId}` === item.href
          const defaultColor = isActive ? 'var(--text-primary)' : 'var(--text-secondary)'
          return (
            <a
              key={item.href}
              href={item.href}
              className="block no-underline"
              style={{
                fontSize: '13px', fontWeight: 475, lineHeight: '15.6px',
                letterSpacing: '-0.04px', padding: '5px 0',
                color: defaultColor, fontFamily: 'var(--font-primary)',
                transition: 'color 0.15s ease',
              }}
              onMouseEnter={(e) => { if (!isActive) e.currentTarget.style.color = 'var(--text-hover)' }}
              onMouseLeave={(e) => { e.currentTarget.style.color = defaultColor }}
            >
              {item.label}
            </a>
          )
        })}
      </nav>
    </aside>
  )
}

/* ===== Top header bar ===== */

function GitHubIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 0C5.374 0 0 5.373 0 12c0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23A11.509 11.509 0 0 1 12 5.803c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576C20.566 21.797 24 17.3 24 12c0-6.627-5.373-12-12-12z" />
    </svg>
  )
}

function TopHeader() {
  const { locale, setLocale, t } = useI18n()

  return (
    <header
      className="mx-auto flex items-center justify-between"
      style={{
        width: '550px',
        maxWidth: 'calc(100% - 2rem)',
        height: '40px',
        padding: '0 16px',
        marginTop: '56px',
        border: '1px solid var(--page-border)',
        borderRadius: '10px',
        background: 'var(--bg)',
      }}
    >
      <a
        href="/"
        className="no-underline"
        style={{
          fontSize: '14px', fontWeight: 700, letterSpacing: '-0.09px',
          color: 'var(--text-primary)', fontFamily: 'var(--font-primary)',
          transition: 'color 0.15s ease',
        }}
        onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--text-hover)' }}
        onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-primary)' }}
      >
        tunelo
      </a>
      <div className="flex items-center gap-1">
        <a
          href="https://github.com/jiweiyuan/tunelo"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-1.5 no-underline"
          style={{
            fontSize: '13px', fontWeight: 500,
            padding: '4px 10px',
            color: 'var(--text-secondary)', fontFamily: 'var(--font-primary)',
            transition: 'color 0.15s ease',
            borderRadius: '6px',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--text-primary)' }}
          onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-secondary)' }}
        >
          <GitHubIcon />
          <span>GitHub</span>
        </a>
        <span style={{ width: '1px', height: '14px', background: 'var(--page-border)' }} />
        <button
          onClick={() => setLocale(locale === 'en' ? 'zh' : 'en')}
          className="cursor-pointer"
          style={{
            fontSize: '13px', fontWeight: 500,
            padding: '4px 10px',
            color: 'var(--text-secondary)', fontFamily: 'var(--font-primary)',
            transition: 'color 0.15s ease',
            background: 'none', border: 'none',
          }}
          onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--text-primary)' }}
          onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-secondary)' }}
        >
          {t('switchLabel')}
        </button>
      </div>
    </header>
  )
}

/* ===== Section heading with divider ===== */

export function SectionHeading({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h1
      id={id}
      className="scroll-mt-[5.25rem]"
      style={{
        fontFamily: 'var(--font-primary)', fontSize: '14px', fontWeight: 560,
        lineHeight: '20px', letterSpacing: '-0.09px', color: 'var(--text-primary)',
        margin: 0, padding: 0, display: 'flex', alignItems: 'center', gap: '12px',
        paddingTop: '24px', paddingBottom: '24px',
      }}
    >
      <span style={{ whiteSpace: 'nowrap' }}>{children}</span>
      <span style={{ flex: 1, height: '1px', background: 'var(--divider)' }} />
    </h1>
  )
}

/* ===== Prose paragraph ===== */

export function P({ children }: { children: React.ReactNode }) {
  return (
    <p className="editorial-prose" style={{ margin: 0 }}>
      {children}
    </p>
  )
}

/* ===== Caption ===== */

export function Caption({ children }: { children: React.ReactNode }) {
  return (
    <p style={{
      fontFamily: 'var(--font-primary)', fontSize: '12px', fontWeight: 475,
      textAlign: 'center', lineHeight: '20px', letterSpacing: '-0.09px',
      color: 'var(--text-secondary)', margin: 0,
    }}>
      {children}
    </p>
  )
}

/* ===== Link ===== */

export function A({ href, children }: { href: string; children: React.ReactNode }) {
  const isAnchor = href.startsWith('#')
  return (
    <a
      href={href}
      target={isAnchor ? undefined : '_blank'}
      rel={isAnchor ? undefined : 'noopener noreferrer'}
      style={{ color: 'var(--link-accent)', fontWeight: 600, textDecoration: 'none' }}
      onMouseEnter={(e) => { e.currentTarget.style.textDecoration = 'underline' }}
      onMouseLeave={(e) => { e.currentTarget.style.textDecoration = 'none' }}
    >
      {children}
    </a>
  )
}

/* ===== Inline code ===== */

export function Code({ children }: { children: React.ReactNode }) {
  return <code className="inline-code">{children}</code>
}

/* ===== Code block with Prism ===== */

export function CodeBlock({
  children,
  lang = 'bash',
  showLineNumbers = true,
}: {
  children: string
  lang?: string
  showLineNumbers?: boolean
}) {
  const codeRef = useRef<HTMLElement>(null)
  const content = typeof children === 'string' ? children : String(children)
  const lines = content.split('\n')

  useEffect(() => {
    if (codeRef.current && lang) Prism.highlightElement(codeRef.current)
  }, [content, lang])

  return (
    <figure className="m-0 bleed">
      <div className="relative">
        <pre className="overflow-x-auto" style={{ borderRadius: '8px', margin: 0, padding: 0 }}>
          <div
            className="flex"
            style={{
              padding: '12px 8px 8px', fontFamily: 'var(--font-code)',
              fontSize: '12px', fontWeight: 400, lineHeight: '1.85',
              letterSpacing: 'normal', color: 'var(--text-primary)', tabSize: 2,
            }}
          >
            {showLineNumbers && (
              <span
                className="select-none shrink-0"
                aria-hidden="true"
                style={{ color: 'var(--code-line-nr)', textAlign: 'right', paddingRight: '20px', width: '36px', userSelect: 'none' }}
              >
                {lines.map((_, i) => <span key={i} className="block">{i + 1}</span>)}
              </span>
            )}
            <code
              ref={codeRef}
              className={lang ? `language-${lang}` : undefined}
              style={{ whiteSpace: 'pre', background: 'none', padding: 0, lineHeight: '1.85' }}
            >
              {content}
            </code>
          </div>
        </pre>
      </div>
    </figure>
  )
}

/* ===== Comparison table ===== */

export function ComparisonTable({
  title,
  headers,
  rows,
}: {
  title?: string
  headers: string[]
  rows: string[][]
}) {
  return (
    <div className="w-full max-w-full overflow-x-auto" style={{ padding: '8px 0' }}>
      {title && (
        <div style={{
          fontFamily: 'var(--font-primary)', fontSize: '11px', fontWeight: 400,
          color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.02em',
          padding: '0 0 6px',
        }}>
          {title}
        </div>
      )}
      <table className="w-full" style={{ borderSpacing: 0, borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            {headers.map((h) => (
              <th
                key={h}
                className="text-left"
                style={{
                  padding: '4.8px 12px 4.8px 0', fontSize: '11px', fontWeight: 400,
                  fontFamily: 'var(--font-primary)', color: 'var(--text-muted)',
                  borderBottom: '1px solid var(--page-border)',
                }}
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr key={i}>
              {row.map((cell, j) => (
                <td
                  key={j}
                  style={{
                    padding: '4.8px 12px 4.8px 0', fontSize: '11px', fontWeight: 500,
                    fontFamily: 'var(--font-code)', color: 'var(--text-primary)',
                    borderBottom: '1px solid var(--page-border)', whiteSpace: 'nowrap',
                  }}
                >
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/* ===== List ===== */

export function List({ children }: { children: React.ReactNode }) {
  return (
    <ul
      className="m-0 pl-5"
      style={{
        fontFamily: 'var(--font-primary)', fontSize: '14px', fontWeight: 475,
        lineHeight: '20px', letterSpacing: '-0.09px', color: 'var(--text-primary)',
        listStyleType: 'disc',
      }}
    >
      {children}
    </ul>
  )
}

export function Li({ children }: { children: React.ReactNode }) {
  return <li style={{ padding: '0 0 8px 12px' }}>{children}</li>
}

/* ===== Section wrapper ===== */

export function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }) {
  return (
    <>
      <SectionHeading id={id}>{title}</SectionHeading>
      {children}
    </>
  )
}

/* ===== Page shell ===== */

export function EditorialPage({
  toc,
  children,
}: {
  toc: Array<{ label: string; href: string }>
  children: React.ReactNode
}) {
  return (
    <div
      className="editorial-page relative min-h-screen overflow-x-hidden"
      style={{ background: 'var(--bg)', color: 'var(--text-primary)', fontFamily: 'var(--font-primary)' }}
    >
      <TableOfContents items={toc} />
      <TopHeader />
      <div className="mx-auto" style={{ width: '550px', maxWidth: 'calc(100% - 2rem)', padding: '0 1rem 6rem' }}>
        <div style={{ height: '24px' }} />
        <article className="flex flex-col gap-[32px]">{children}</article>
      </div>
    </div>
  )
}

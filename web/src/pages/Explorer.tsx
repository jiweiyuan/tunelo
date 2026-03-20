import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { dirQueryOptions } from "@/lib/query";
import { detectViewer, formatSize, fileTypeColor } from "@/types";
import { rawUrl } from "@/api";
import {
  Folder,
  CaretRight,
  ArrowLeft,
  File,
  FileCode,
  FileText,
  FilePdf,
  FileImage,
  FileVideo,
  FileAudio,
  FileCsv,
  FileHtml,
  House,
} from "@phosphor-icons/react";

export default function Explorer() {
  const { path } = useSearch({ from: "/" });
  const navigate = useNavigate();
  const { data: entries, isLoading, error } = useQuery(dirQueryOptions(path));

  const goDir = (p: string) => navigate({ to: "/", search: { path: p } });
  const goFile = (p: string) => navigate({ to: "/view", search: { path: p } });

  const parts = path.split("/").filter(Boolean);
  const crumbs = [
    { name: "All files", path: "/" },
    ...parts.map((p, i) => ({
      name: p,
      path: "/" + parts.slice(0, i + 1).join("/") + "/",
    })),
  ];

  return (
    <div className="min-h-screen view-enter" style={{ background: "var(--dropbox-gray-50)" }}>
      {/* ── Breadcrumbs ─────────────────────────────────── */}
      <div style={{ maxWidth: 1120, margin: "0 auto", padding: "24px 24px 4px" }}>
        <nav style={{ display: "flex", alignItems: "center", gap: 4, flexWrap: "wrap" }}>
          {path !== "/" && (
            <button
              className="btn-ghost"
              onClick={() => {
                const parent = path.replace(/\/$/, "").split("/").slice(0, -1).join("/") || "/";
                goDir(parent.endsWith("/") ? parent : parent + "/");
              }}
              style={{ padding: "8px 10px", marginRight: 4 }}
            >
              <ArrowLeft size={16} weight="bold" />
            </button>
          )}
          {crumbs.map((bc, i) => (
            <span key={bc.path} style={{ display: "flex", alignItems: "center", gap: 4 }}>
              {i > 0 && <CaretRight size={12} weight="bold" style={{ color: "var(--dropbox-gray-500)" }} />}
              {i === crumbs.length - 1 ? (
                <span style={{ fontSize: 15, fontWeight: 600, color: "var(--dropbox-gray-900)", padding: "6px 4px" }}>
                  {bc.name}
                </span>
              ) : (
                <button className="crumb-btn" onClick={() => goDir(bc.path)}>
                  {i === 0 ? <House size={15} weight="fill" /> : bc.name}
                </button>
              )}
            </span>
          ))}
        </nav>
      </div>

      {/* ── Content ─────────────────────────────────────── */}
      <div style={{ maxWidth: 1120, margin: "0 auto", padding: "16px 24px 40px" }}>
        {isLoading && <LoadingSkeleton />}

        {error && (
          <div style={{ textAlign: "center", padding: "100px 0" }}>
            <p style={{ fontSize: 15, color: "var(--dropbox-red)" }}>{error.message}</p>
          </div>
        )}

        {!isLoading && !error && entries && entries.length > 0 && (
          <div style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
            gap: 16,
          }}>
            {entries.map((entry, i) => {
              const fullPath = (path.endsWith("/") ? path : path + "/") + entry.name;
              return (
                <div
                  key={entry.name}
                  className="file-card stagger-item"
                  style={{ animationDelay: `${Math.min(i * 40, 400)}ms` }}
                  onClick={() => entry.is_dir ? goDir(fullPath + "/") : goFile(fullPath)}
                >
                  <div className="card-preview">
                    {entry.is_dir ? (
                      <Folder size={56} weight="fill" style={{ color: "#F7BE00" }} />
                    ) : isImageFile(entry.name) ? (
                      <img src={rawUrl(fullPath)} alt={entry.name} loading="lazy" />
                    ) : (
                      <FileIcon name={entry.name} size={48} />
                    )}
                  </div>
                  <div className="card-info">
                    <div className="card-name">{entry.name}</div>
                    <div className="card-meta">
                      {entry.is_dir ? "Folder" : formatSize(entry.size)}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {!isLoading && !error && entries && entries.length === 0 && (
          <div style={{ textAlign: "center", padding: "100px 0", color: "var(--dropbox-gray-500)" }}>
            <Folder size={64} weight="thin" style={{ margin: "0 auto 16px", display: "block", color: "var(--dropbox-gray-300)" }} />
            <p style={{ fontSize: 16, fontWeight: 500 }}>This folder is empty</p>
          </div>
        )}
      </div>

      {/* ── Footer ──────────────────────────────────────── */}
      <div style={{
        maxWidth: 1120, margin: "0 auto", padding: "0 24px 24px",
        display: "flex", justifyContent: "flex-end", alignItems: "center",
      }}>
        <a
          href="https://tunelo.net"
          target="_blank"
          rel="noopener noreferrer"
          style={{ fontSize: 12, color: "var(--dropbox-gray-500)", textDecoration: "none" }}
        >
          Served by{" "}
          <span style={{ fontWeight: 600, color: "var(--dropbox-blue)" }}>tunelo</span>
        </a>
      </div>
    </div>
  );
}

/* ── File icon ────────────────────────────────────────── */
function FileIcon({ name, size }: { name: string; size: number }) {
  const kind = detectViewer(name, false);
  const color = fileTypeColor(name, false);
  const props = { size, weight: "duotone" as const, style: { color } };

  switch (kind) {
    case "code": return <FileCode {...props} />;
    case "markdown":
    case "text": return <FileText {...props} />;
    case "pdf": return <FilePdf {...props} />;
    case "image": return <FileImage {...props} />;
    case "video": return <FileVideo {...props} />;
    case "audio": return <FileAudio {...props} />;
    case "csv":
    case "excel": return <FileCsv {...props} />;
    case "html": return <FileHtml {...props} />;
    default: return <File {...props} />;
  }
}

function isImageFile(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  return ["jpg", "jpeg", "png", "gif", "webp", "avif", "bmp"].includes(ext);
}

/* ── Skeleton ─────────────────────────────────────────── */
function LoadingSkeleton() {
  return (
    <div style={{
      display: "grid",
      gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
      gap: 16,
    }}>
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} className="stagger-item" style={{
          animationDelay: `${i * 40}ms`,
          background: "var(--dropbox-white)",
          borderRadius: 12,
          overflow: "hidden",
          boxShadow: "0 1px 3px rgba(0,0,0,0.04), 0 0 0 1px rgba(0,0,0,0.06)",
        }}>
          <div style={{ height: 160, background: "var(--dropbox-gray-100)", animation: "pulse 1.5s ease-in-out infinite" }} />
          <div style={{ padding: "14px 16px" }}>
            <div style={{ height: 14, width: `${50 + Math.random() * 40}%`, background: "var(--dropbox-gray-200)", borderRadius: 4 }} />
            <div style={{ height: 12, width: "30%", background: "var(--dropbox-gray-200)", borderRadius: 4, marginTop: 8 }} />
          </div>
        </div>
      ))}
    </div>
  );
}

import { Fragment, useEffect, useId, useState } from 'react';
import {
  ChevronDown,
  ChevronUp,
  Columns2,
  ExternalLink,
  FileCode2,
  FileWarning,
  List,
  Loader2,
  Minus,
  Plus,
} from 'lucide-react';
import { useTheme } from '../../contexts/ThemeContext';

export type ContributionDiffViewMode = 'side-by-side' | 'inline';
export type ContributionDiffStatus = 'ready' | 'loading-diff' | 'unsupported-preview';
export type ContributionDiffSideKind = 'context' | 'added' | 'removed' | 'empty';

export interface ContributionDiffSide {
  kind: ContributionDiffSideKind;
  lineNumber?: number;
  content: string;
}

export interface ContributionDiffRow {
  id: string;
  left: ContributionDiffSide;
  right: ContributionDiffSide;
}

export interface ContributionDiffCollapsedHunk {
  kind: 'collapsed-hunk';
  id: string;
  unchangedLines: number;
  rows: ContributionDiffRow[];
}

export type ContributionDiffHunkRow = ContributionDiffRow | ContributionDiffCollapsedHunk;

export interface ContributionDiffHunk {
  id: string;
  header: string;
  rows: ContributionDiffHunkRow[];
}

export interface ContributionDiffFile {
  path: string;
  previousPath?: string;
  additions: number;
  deletions: number;
  isBinary?: boolean;
  isPartial?: boolean;
  hunks: ContributionDiffHunk[];
}

export interface ContributionDiff {
  title: string;
  number: number;
  url?: string;
  author?: {
    name: string;
    avatar?: string;
  };
  additions: number;
  deletions: number;
  changedFiles: number;
  files: ContributionDiffFile[];
}

export interface ContributionDiffViewerProps {
  // Keep the UI contract independent from GitHub response fields so an API adapter can map it later.
  diff?: ContributionDiff | null;
  status?: ContributionDiffStatus;
  defaultViewMode?: ContributionDiffViewMode;
  onLoadFullFile?: (path: string) => void | Promise<void>;
}

interface DiffFileProps {
  file: ContributionDiffFile;
  mode: ContributionDiffViewMode;
  isDark: boolean;
  expandedHunks: Set<string>;
  onToggleHunk: (id: string) => void;
  onLoadFullFile?: (path: string) => void | Promise<void>;
  loadingFilePath: string | null;
}

function isCompactViewport(): boolean {
  return typeof window !== 'undefined' && window.matchMedia?.('(max-width: 767px)').matches === true;
}

function getSurfaceClass(kind: ContributionDiffSideKind, isDark: boolean): string {
  if (kind === 'added') return isDark ? 'bg-[#22c55e]/10' : 'bg-[#f0fdf4]';
  if (kind === 'removed') return isDark ? 'bg-[#ff6e6e]/10' : 'bg-[#fef2f2]';
  if (kind === 'empty') return isDark ? 'bg-white/[0.02]' : 'bg-black/[0.02]';
  return isDark ? 'bg-[#2d2820]' : 'bg-[#fafaf9]';
}

function getMarkerClass(kind: ContributionDiffSideKind, isDark: boolean): string {
  if (kind === 'added') return isDark ? 'text-[#22c55e]' : 'text-[#15803d]';
  if (kind === 'removed') return isDark ? 'text-[#ff6e6e]' : 'text-[#b91c1c]';
  return isDark ? 'text-[#b8a898]' : 'text-[#78716c]';
}

function getKindLabel(kind: ContributionDiffSideKind): string {
  if (kind === 'added') return 'Added line';
  if (kind === 'removed') return 'Removed line';
  if (kind === 'context') return 'Unchanged line';
  return 'Empty diff side';
}

function getKindPrefix(kind: ContributionDiffSideKind): string {
  if (kind === 'added') return '+';
  if (kind === 'removed') return '-';
  return ' ';
}

function isCollapsedHunk(row: ContributionDiffHunkRow): row is ContributionDiffCollapsedHunk {
  return 'kind' in row && row.kind === 'collapsed-hunk';
}

function DiffMarker({ kind, isDark }: { kind: ContributionDiffSideKind; isDark: boolean }) {
  if (kind === 'empty') return <span aria-hidden="true" className="inline-block w-4" />;

  const Icon = kind === 'added' ? Plus : kind === 'removed' ? Minus : null;
  return (
    <span className={`inline-flex w-4 shrink-0 items-center justify-center ${getMarkerClass(kind, isDark)}`}>
      {Icon ? <Icon aria-hidden="true" className="h-3 w-3" strokeWidth={2.5} /> : <span aria-hidden="true" />}
      <span className="sr-only">{getKindLabel(kind)}</span>
    </span>
  );
}

function DiffCode({ side, isDark }: { side: ContributionDiffSide; isDark: boolean }) {
  if (side.kind === 'empty') return <span aria-hidden="true" />;

  return (
    <code
      aria-label={`${getKindLabel(side.kind)}: ${side.content || 'blank line'}`}
      className={`block whitespace-pre font-mono text-[12px] leading-6 ${isDark ? 'text-[#f5f5f5]' : 'text-[#292524]'}`}
    >
      <span aria-hidden="true" className={getMarkerClass(side.kind, isDark)}>
        {getKindPrefix(side.kind)}{' '}
      </span>
      {side.content || ' '}
    </code>
  );
}

function LineNumber({ side, isDark }: { side: ContributionDiffSide; isDark: boolean }) {
  return (
    <span className={`block min-w-10 select-none px-2 text-right font-mono text-[11px] leading-6 ${
      isDark ? 'text-[#b8a898]' : 'text-[#78716c]'
    }`}>
      {side.lineNumber ?? ''}
    </span>
  );
}

function HunkMarker({
  hunk,
  mode,
  isDark,
  isExpanded,
  onToggle,
}: {
  hunk: ContributionDiffCollapsedHunk;
  mode: ContributionDiffViewMode;
  isDark: boolean;
  isExpanded: boolean;
  onToggle: () => void;
}) {
  return (
    <tr>
      <td colSpan={mode === 'side-by-side' ? 4 : 3} className="p-0">
        <button
          type="button"
          aria-expanded={isExpanded}
          aria-controls={`${hunk.id}-rows`}
          onClick={onToggle}
          className={`flex min-h-10 w-full items-center justify-center gap-2 border-y px-3 py-2 text-left text-[12px] font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f1b400] focus-visible:ring-inset ${
            isDark
              ? 'border-white/10 bg-white/[0.06] text-[#e8dfd0] hover:bg-white/[0.1]'
              : 'border-black/10 bg-black/[0.03] text-[#4a3f2f] hover:bg-black/[0.06]'
          }`}
        >
          {isExpanded ? <ChevronUp aria-hidden="true" className="h-4 w-4" /> : <ChevronDown aria-hidden="true" className="h-4 w-4" />}
          <span>{isExpanded ? 'Collapse unchanged lines' : `+${hunk.unchangedLines} lines unchanged`}</span>
          {!isExpanded && <span id={`${hunk.id}-rows`} aria-hidden="true" className="sr-only">Unchanged lines are collapsed</span>}
        </button>
      </td>
    </tr>
  );
}

function SideBySideRow({ row, isDark, id }: { row: ContributionDiffRow; isDark: boolean; id?: string }) {
  return (
    <tr id={id}>
      <td className={`w-10 border-r border-black/10 p-0 align-top ${getSurfaceClass(row.left.kind, isDark)}`}>
        <LineNumber side={row.left} isDark={isDark} />
      </td>
      <td className={`min-w-[280px] border-r border-black/10 p-0 align-top ${getSurfaceClass(row.left.kind, isDark)}`}>
        <div className="flex min-h-6 items-start px-2">
          <DiffMarker kind={row.left.kind} isDark={isDark} />
          <DiffCode side={row.left} isDark={isDark} />
        </div>
      </td>
      <td className={`w-10 border-r border-black/10 p-0 align-top ${getSurfaceClass(row.right.kind, isDark)}`}>
        <LineNumber side={row.right} isDark={isDark} />
      </td>
      <td className={`min-w-[280px] p-0 align-top ${getSurfaceClass(row.right.kind, isDark)}`}>
        <div className="flex min-h-6 items-start px-2">
          <DiffMarker kind={row.right.kind} isDark={isDark} />
          <DiffCode side={row.right} isDark={isDark} />
        </div>
      </td>
    </tr>
  );
}

function InlineRow({ side, isDark, id }: { side: ContributionDiffSide; isDark: boolean; id?: string }) {
  return (
    <tr id={id}>
      <td className={`w-10 border-r border-black/10 p-0 align-top ${getSurfaceClass(side.kind, isDark)}`}>
        <LineNumber side={side} isDark={isDark} />
      </td>
      <td className={`w-8 p-0 align-top ${getSurfaceClass(side.kind, isDark)}`}>
        <div className="flex min-h-6 items-start justify-center pt-1">
          <DiffMarker kind={side.kind} isDark={isDark} />
        </div>
      </td>
      <td className={`min-w-[360px] p-0 align-top ${getSurfaceClass(side.kind, isDark)}`}>
        <div className="min-h-6 px-2">
          <DiffCode side={side} isDark={isDark} />
        </div>
      </td>
    </tr>
  );
}

function SideBySideDiff({
  file,
  isDark,
  expandedHunks,
  onToggleHunk,
}: Pick<DiffFileProps, 'file' | 'isDark' | 'expandedHunks' | 'onToggleHunk'>) {
  return (
    <div className="overflow-x-auto">
      <table className="min-w-[640px] w-full border-collapse text-left" aria-label={`Side-by-side diff for ${file.path}`}>
        <thead>
          <tr className={isDark ? 'bg-white/[0.04] text-[#b8a898]' : 'bg-black/[0.03] text-[#78716c]'}>
            <th colSpan={2} scope="colgroup" className="border-r border-black/10 px-3 py-2 text-[10px] font-bold uppercase tracking-wide">Original</th>
            <th colSpan={2} scope="colgroup" className="px-3 py-2 text-[10px] font-bold uppercase tracking-wide">Changed</th>
          </tr>
        </thead>
        {file.hunks.map((hunk) => (
          <tbody key={hunk.id}>
            <tr className={isDark ? 'bg-[#3a3428] text-[#e8dfd0]' : 'bg-[#e7e5e4] text-[#44403c]'}>
              <th colSpan={4} scope="rowgroup" className="px-3 py-1.5 text-left font-mono text-[11px] font-medium">{hunk.header}</th>
            </tr>
            {hunk.rows.map((row) => {
              if (isCollapsedHunk(row)) {
                const isExpanded = expandedHunks.has(row.id);
                return (
                  <Fragment key={row.id}>
                    <HunkMarker hunk={row} mode="side-by-side" isDark={isDark} isExpanded={isExpanded} onToggle={() => onToggleHunk(row.id)} />
                    {isExpanded && row.rows.map((expandedRow, index) => <SideBySideRow key={expandedRow.id} id={index === 0 ? `${row.id}-rows` : undefined} row={expandedRow} isDark={isDark} />)}
                  </Fragment>
                );
              }
              return <SideBySideRow key={row.id} row={row} isDark={isDark} />;
            })}
          </tbody>
        ))}
      </table>
    </div>
  );
}

function InlineDiff({
  file,
  isDark,
  expandedHunks,
  onToggleHunk,
}: Pick<DiffFileProps, 'file' | 'isDark' | 'expandedHunks' | 'onToggleHunk'>) {
  return (
    <div className="overflow-x-auto">
      <table className="min-w-[400px] w-full border-collapse text-left" aria-label={`Inline diff for ${file.path}`}>
        <thead>
          <tr className={isDark ? 'bg-white/[0.04] text-[#b8a898]' : 'bg-black/[0.03] text-[#78716c]'}>
            <th scope="col" className="px-3 py-2 text-[10px] font-bold uppercase tracking-wide">Line</th>
            <th scope="col" className="px-2 py-2 text-[10px] font-bold uppercase tracking-wide">Change</th>
            <th scope="col" className="px-3 py-2 text-[10px] font-bold uppercase tracking-wide">Code</th>
          </tr>
        </thead>
        {file.hunks.map((hunk) => (
          <tbody key={hunk.id}>
            <tr className={isDark ? 'bg-[#3a3428] text-[#e8dfd0]' : 'bg-[#e7e5e4] text-[#44403c]'}>
              <th colSpan={3} scope="rowgroup" className="px-3 py-1.5 text-left font-mono text-[11px] font-medium">{hunk.header}</th>
            </tr>
            {hunk.rows.map((row) => {
              if (isCollapsedHunk(row)) {
                const isExpanded = expandedHunks.has(row.id);
                return (
                  <Fragment key={row.id}>
                    <HunkMarker hunk={row} mode="inline" isDark={isDark} isExpanded={isExpanded} onToggle={() => onToggleHunk(row.id)} />
                    {isExpanded && row.rows.flatMap((expandedRow) => [
                      expandedRow.left.kind !== 'empty' ? <InlineRow key={`${expandedRow.id}-left`} id={expandedRow.id === row.rows[0]?.id ? `${row.id}-rows` : undefined} side={expandedRow.left} isDark={isDark} /> : null,
                      expandedRow.right.kind !== 'empty' && expandedRow.right.kind !== 'context' ? <InlineRow key={`${expandedRow.id}-right`} id={expandedRow.id === row.rows[0]?.id && expandedRow.left.kind === 'empty' ? `${row.id}-rows` : undefined} side={expandedRow.right} isDark={isDark} /> : null,
                    ])}
                  </Fragment>
                );
              }
              return (
                <Fragment key={`${row.id}-inline`}>
                  {row.left.kind !== 'empty' && <InlineRow side={row.left} isDark={isDark} />}
                  {row.right.kind !== 'empty' && row.right.kind !== 'context' && <InlineRow side={row.right} isDark={isDark} />}
                </Fragment>
              );
            })}
          </tbody>
        ))}
      </table>
    </div>
  );
}

function DiffFile({
  file,
  mode,
  isDark,
  expandedHunks,
  onToggleHunk,
  onLoadFullFile,
  loadingFilePath,
}: DiffFileProps) {
  const regionId = `diff-file-${file.path.replace(/[^a-z0-9]+/gi, '-')}`;
  const isLoading = loadingFilePath === file.path;

  return (
    <section
      role="region"
      aria-label={`Diff for ${file.path}`}
      aria-labelledby={`${regionId}-heading`}
      className={`overflow-hidden rounded-[12px] border ${isDark ? 'border-white/10 bg-[#2d2820]' : 'border-black/10 bg-[#fafaf9]'}`}
    >
      <header className={`flex flex-wrap items-center justify-between gap-3 border-b px-4 py-3 ${isDark ? 'border-white/10 bg-white/[0.05]' : 'border-black/10 bg-white/70'}`}>
        <div className="flex min-w-0 items-center gap-2">
          <FileCode2 aria-hidden="true" className={`h-4 w-4 shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`} />
          <div className="min-w-0">
            <h3 id={`${regionId}-heading`} className={`truncate font-mono text-[12px] font-semibold ${isDark ? 'text-[#f5f5f5]' : 'text-[#292524]'}`}>
              {file.path}
            </h3>
            {file.previousPath && <p className={`truncate text-[11px] ${isDark ? 'text-[#b8a898]' : 'text-[#78716c]'}`}>renamed from {file.previousPath}</p>}
          </div>
        </div>
        <div className="flex items-center gap-3 text-[11px] font-semibold" aria-label={`${file.additions} additions, ${file.deletions} deletions`}>
          <span className={isDark ? 'text-[#22c55e]' : 'text-[#15803d]'}>+{file.additions}</span>
          <span className={isDark ? 'text-[#ff6e6e]' : 'text-[#b91c1c]'}>-{file.deletions}</span>
        </div>
      </header>

      {file.isBinary ? (
        <div className={`flex items-center gap-3 px-4 py-8 text-[13px] ${isDark ? 'text-[#d4d4d4]' : 'text-[#57534e]'}`} role="status">
          <FileWarning aria-hidden="true" className="h-5 w-5 shrink-0" />
          <span>Binary file preview is not supported.</span>
        </div>
      ) : (
        <>
          {mode === 'side-by-side' ? (
            <SideBySideDiff file={file} isDark={isDark} expandedHunks={expandedHunks} onToggleHunk={onToggleHunk} />
          ) : (
            <InlineDiff file={file} isDark={isDark} expandedHunks={expandedHunks} onToggleHunk={onToggleHunk} />
          )}
          {file.isPartial && (
            <footer className={`flex items-center justify-end border-t px-4 py-3 ${isDark ? 'border-white/10 bg-white/[0.03]' : 'border-black/10 bg-black/[0.02]'}`}>
              <button
                type="button"
                disabled={isLoading || !onLoadFullFile}
                aria-busy={isLoading}
                onClick={() => {
                  if (onLoadFullFile) void onLoadFullFile(file.path);
                }}
                className={`inline-flex min-h-10 items-center gap-2 rounded-[8px] border px-3 py-2 text-[12px] font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f1b400] focus-visible:ring-offset-2 disabled:cursor-wait disabled:opacity-60 ${isDark ? 'border-white/15 bg-white/[0.08] text-[#e8dfd0] hover:bg-white/[0.12]' : 'border-black/10 bg-white text-[#4a3f2f] hover:bg-black/[0.04]'}`}
              >
                {isLoading ? <Loader2 aria-hidden="true" className="h-4 w-4 animate-spin" /> : null}
                {isLoading ? 'Loading full file' : 'Load full file'}
              </button>
            </footer>
          )}
        </>
      )}
    </section>
  );
}

function LoadingDiff({ isDark }: { isDark: boolean }) {
  return (
    <div className="space-y-3" role="status" aria-live="polite">
      <span className="sr-only">Loading diff</span>
      {[0, 1].map((item) => (
        <div key={item} className={`overflow-hidden rounded-[12px] border ${isDark ? 'border-white/10 bg-[#2d2820]' : 'border-black/10 bg-[#fafaf9]'}`}>
          <div className={`h-12 animate-pulse ${isDark ? 'bg-white/[0.08]' : 'bg-black/[0.06]'}`} />
          <div className="space-y-2 p-3">
            {[0, 1, 2, 3, 4].map((line) => <div key={line} className={`h-6 animate-pulse rounded ${isDark ? 'bg-white/[0.06]' : 'bg-black/[0.05]'}`} />)}
        </div>
      </div>
      ))}
    </div>
  );
}

function UnsupportedPreview({ diff, isDark }: { diff?: ContributionDiff | null; isDark: boolean }) {
  return (
    <div className={`rounded-[12px] border px-4 py-8 text-center ${isDark ? 'border-white/10 bg-white/[0.04] text-[#d4d4d4]' : 'border-black/10 bg-white/60 text-[#57534e]'}`} role="status">
      <FileWarning aria-hidden="true" className="mx-auto mb-3 h-6 w-6" />
      <p className="text-[13px] font-semibold">Diff preview unavailable</p>
      <p className="mx-auto mt-1 max-w-md text-[12px]">The pull request is linked, but its patch is not available in the current data response.</p>
      {diff?.url && (
        <a href={diff.url} target="_blank" rel="noopener noreferrer" className={`mt-4 inline-flex items-center gap-1 text-[12px] font-semibold underline ${isDark ? 'text-[#e8c77f]' : 'text-[#8b6527]'}`}>
          Open pull request on GitHub <ExternalLink aria-hidden="true" className="h-3 w-3" />
        </a>
      )}
    </div>
  );
}

export function ContributionDiffViewer({
  diff,
  status = 'ready',
  defaultViewMode,
  onLoadFullFile,
}: ContributionDiffViewerProps) {
  const { theme } = useTheme();
  const isDark = theme === 'dark';
  const [isCompact, setIsCompact] = useState(isCompactViewport);
  const [manualViewMode, setManualViewMode] = useState<ContributionDiffViewMode | null>(defaultViewMode ?? null);
  const [expandedHunks, setExpandedHunks] = useState<Set<string>>(new Set());
  const [loadingFilePath, setLoadingFilePath] = useState<string | null>(null);
  const viewerId = useId().replace(/:/g, '');

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return undefined;
    const query = window.matchMedia('(max-width: 767px)');
    const handleChange = (event: MediaQueryListEvent) => setIsCompact(event.matches);
    query.addEventListener('change', handleChange);
    return () => query.removeEventListener('change', handleChange);
  }, []);

  const viewMode = manualViewMode ?? (isCompact ? 'inline' : 'side-by-side');

  const toggleHunk = (id: string) => {
    setExpandedHunks((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const loadFullFile = async (path: string) => {
    if (!onLoadFullFile) return;
    setLoadingFilePath(path);
    try {
      await onLoadFullFile(path);
    } finally {
      setLoadingFilePath(null);
    }
  };

  return (
    <section
      role="region"
      aria-label="Contribution diff viewer"
      aria-busy={status === 'loading-diff'}
      aria-labelledby={`${viewerId}-heading`}
      className={`space-y-4 rounded-[16px] border p-4 shadow-[0_4px_18px_rgba(0,0,0,0.08)] ${isDark ? 'border-white/10 bg-white/[0.06]' : 'border-white/25 bg-white/[0.15]'}`}
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <FileCode2 aria-hidden="true" className={`h-5 w-5 shrink-0 ${isDark ? 'text-[#c9983a]' : 'text-[#a67c2e]'}`} />
            <h2 id={`${viewerId}-heading`} className={`truncate text-[16px] font-bold ${isDark ? 'text-[#f5f5f5]' : 'text-[#2d2820]'}`}>Contribution diff</h2>
          </div>
          {diff && status === 'ready' && (
            <p className={`mt-1 text-[12px] ${isDark ? 'text-[#b8a898]' : 'text-[#78716c]'}`}>
              #{diff.number} {diff.title} | {diff.changedFiles} {diff.changedFiles === 1 ? 'file' : 'files'} changed | <span className={isDark ? 'text-[#22c55e]' : 'text-[#15803d]'}>+{diff.additions}</span> <span className={isDark ? 'text-[#ff6e6e]' : 'text-[#b91c1c]'}>-{diff.deletions}</span>
              {diff.author && <span> | by {diff.author.name}</span>}
            </p>
          )}
        </div>

        <div className="flex flex-wrap items-center justify-end gap-2">
          <div role="group" aria-label="Diff view mode" className={`inline-flex rounded-[8px] border p-0.5 ${isDark ? 'border-white/15 bg-white/[0.05]' : 'border-black/10 bg-white/70'}`}>
            <button
              type="button"
              aria-label="Side-by-side view"
              aria-pressed={viewMode === 'side-by-side'}
              onClick={() => setManualViewMode('side-by-side')}
              className={`inline-flex min-h-10 items-center gap-1.5 rounded-[6px] px-2.5 text-[11px] font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f1b400] ${viewMode === 'side-by-side' ? (isDark ? 'bg-[#c9983a] text-[#1a1714]' : 'bg-[#a67c2e] text-white') : (isDark ? 'text-[#d4d4d4] hover:bg-white/[0.08]' : 'text-[#57534e] hover:bg-black/[0.04]')}`}>
              <Columns2 aria-hidden="true" className="h-3.5 w-3.5" />
              <span>Side-by-side</span>
            </button>
            <button
              type="button"
              aria-label="Inline view"
              aria-pressed={viewMode === 'inline'}
              onClick={() => setManualViewMode('inline')}
              className={`inline-flex min-h-10 items-center gap-1.5 rounded-[6px] px-2.5 text-[11px] font-semibold transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#f1b400] ${viewMode === 'inline' ? (isDark ? 'bg-[#c9983a] text-[#1a1714]' : 'bg-[#a67c2e] text-white') : (isDark ? 'text-[#d4d4d4] hover:bg-white/[0.08]' : 'text-[#57534e] hover:bg-black/[0.04]')}`}>
              <List aria-hidden="true" className="h-3.5 w-3.5" />
              <span>Inline</span>
            </button>
          </div>
          {diff?.url && (
            <a href={diff.url} target="_blank" rel="noopener noreferrer" className={`inline-flex min-h-10 items-center gap-1 rounded-[8px] border px-3 py-2 text-[11px] font-semibold ${isDark ? 'border-white/15 text-[#e8c77f] hover:bg-white/[0.08]' : 'border-black/10 text-[#8b6527] hover:bg-black/[0.04]'}`}>
              GitHub <ExternalLink aria-hidden="true" className="h-3.5 w-3.5" />
            </a>
          )}
        </div>
      </header>

      {status === 'loading-diff' ? <LoadingDiff isDark={isDark} /> : status === 'unsupported-preview' || !diff ? <UnsupportedPreview diff={diff} isDark={isDark} /> : (
        // Do not invent sample content when the upstream response has no patch.
        <div className="space-y-3" aria-label="Changed files">
          {diff.files.map((file) => (
            <DiffFile
              key={file.path}
              file={file}
              mode={viewMode}
              isDark={isDark}
              expandedHunks={expandedHunks}
              onToggleHunk={toggleHunk}
              onLoadFullFile={onLoadFullFile ? loadFullFile : undefined}
              loadingFilePath={loadingFilePath}
            />
          ))}
        </div>
      )}
    </section>
  );
}

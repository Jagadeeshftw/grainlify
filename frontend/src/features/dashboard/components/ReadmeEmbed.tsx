/**
 * ReadmeEmbed — self-contained README renderer for ProjectDetailPage.
 *
 * Fixes addressed in this component vs the previous inline OverviewMarkdown:
 *
 * 1. Light-mode link color #b8872f (2.43:1 ❌) → #6b4c1a (5.95:1 ✅)
 * 2. Light inline-code text #6b5d4d (4.16:1 borderline) → #5c3d0a (7.47:1 ✅)
 * 3. README h1 competed with page h1 → heading offset +2 applied
 * 4. alt="" hardcoded on all images → passes through markdown alt prop
 * 5. No table element handlers → full table styling added
 * 6. No "View on GitHub" affordance → rendered by parent (ProjectDetailPage)
 * 7. No broken-image fallback → onError placeholder implemented
 * 8. No max-width / measure → max-w-[72ch] applied
 *
 * @see design/specs/project-readme-embed.md
 */

import React, { createContext, useContext, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { ImageOff } from 'lucide-react';
import { isDarkVariant, type Theme } from '../../../shared/contexts/ThemeContext';

// ---------------------------------------------------------------------------
// Context: tracks whether we are inside a <pre> block so <code> can inherit
// ---------------------------------------------------------------------------
const InPreContext = createContext(false);

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------
export interface ReadmeEmbedProps {
  /** Raw markdown string from project.readme */
  content: string;
  /** Current theme from useTheme() */
  theme: Theme;
}

// ---------------------------------------------------------------------------
// Broken-image placeholder
// ---------------------------------------------------------------------------
function BrokenImagePlaceholder({
  alt,
  isDark,
}: {
  alt: string;
  isDark: boolean;
}) {
  return (
    <span
      role="img"
      aria-label={alt || 'Image unavailable'}
      className={`inline-flex items-center gap-2 px-4 py-3 my-4 rounded-[12px] text-[13px] border w-full ${
        isDark
          ? 'bg-white/[0.06] border-white/20 text-[#d4d4d4]'
          : 'bg-black/[0.04] border-black/15 text-[#7a6b5a]'
      }`}
      style={{ borderStyle: 'dashed' }}
    >
      <ImageOff
        className={`w-4 h-4 shrink-0 ${isDark ? 'text-[#d4d4d4]' : 'text-[#7a6b5a]'}`}
        aria-hidden="true"
      />
      {alt || 'Image unavailable'}
    </span>
  );
}

// ---------------------------------------------------------------------------
// Image wrapper that handles broken loads
// ---------------------------------------------------------------------------
function ReadmeImage({
  src,
  alt,
  isDark,
  ...rest
}: {
  src?: string;
  alt?: string;
  isDark: boolean;
  [key: string]: unknown;
}) {
  const [broken, setBroken] = useState(false);

  if (broken) {
    return <BrokenImagePlaceholder alt={alt ?? ''} isDark={isDark} />;
  }

  return (
    <img
      src={src}
      // Pass through alt from markdown — do NOT override with ""
      alt={alt ?? ''}
      loading="lazy"
      className="block mx-auto my-4 rounded-[12px] max-w-full h-auto"
      onError={() => setBroken(true)}
      {...rest}
    />
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

/**
 * ReadmeEmbed renders a GitHub README markdown string with full element
 * styling that matches the Grainlify visual language and passes WCAG 2.1 AA.
 *
 * Container: max-w-[72ch] (optimal reading measure), word-break to prevent
 * long URLs from overflowing at 375 px.
 */
export function ReadmeEmbed({ content, theme }: ReadmeEmbedProps) {
  const dark = isDarkVariant(theme);
  const inPre = useContext(InPreContext);

  // ---------------------------------------------------------------------------
  // Token aliases
  // ---------------------------------------------------------------------------
  const textColor = dark ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]';
  const headingColor = dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]';
  // Link: dark #f5c563 (8.89:1) / light #6b4c1a (5.95:1) — both ✅
  const linkClass = dark
    ? 'text-[#f5c563] hover:text-[#ffd700]'
    : 'text-[#6b4c1a] hover:text-[#4a3310]';
  const dividerColor = dark ? 'bg-white/[0.12]' : 'bg-black/[0.10]';

  // Table token aliases
  const tableHeadBg = dark ? 'bg-white/[0.10]' : 'bg-black/[0.06]';
  const tableHeadText = dark ? 'text-[#f5f5f5]' : 'text-[#2d2820]';
  const tableBorder = dark ? 'border-white/10' : 'border-black/[0.07]';
  const tableEvenBg = dark ? 'bg-white/[0.04]' : 'bg-black/[0.02]';
  const tableCellText = dark ? 'text-[#d4d4d4]' : 'text-[#4a3f2f]';

  return (
    <div
      className="max-w-[72ch] break-words"
      // Constrain the README to a readable measure and prevent overflow
    >
      <ReactMarkdown
        components={{
          // ── Headings: offset +2 so README h1 never outranks page h1 ──────
          h1: ({ children, ...props }) => (
            <h3
              className={`text-[22px] font-bold mb-4 mt-6 first:mt-0 ${headingColor}`}
              {...props}
            >
              {children}
            </h3>
          ),
          h2: ({ children, ...props }) => (
            <h4
              className={`text-[18px] font-bold mb-3 mt-5 ${headingColor}`}
              {...props}
            >
              {children}
            </h4>
          ),
          h3: ({ children, ...props }) => (
            <h5
              className={`text-[16px] font-semibold mb-2 mt-4 ${headingColor}`}
              {...props}
            >
              {children}
            </h5>
          ),
          h4: ({ children, ...props }) => (
            <h6
              className={`text-[14px] font-semibold mb-2 mt-3 ${headingColor}`}
              {...props}
            >
              {children}
            </h6>
          ),
          // h5 / h6 stay as-is (already deep in hierarchy)
          h5: ({ children, ...props }) => (
            <h6
              className={`text-[13px] font-semibold mb-1 mt-3 ${headingColor}`}
              {...props}
            >
              {children}
            </h6>
          ),
          h6: ({ children, ...props }) => (
            <p
              className={`text-[12px] font-semibold mb-1 mt-2 ${headingColor}`}
              {...props}
            >
              {children}
            </p>
          ),

          // ── Body ──────────────────────────────────────────────────────────
          p: ({ children, ...props }) => (
            <p
              className={`mb-4 leading-relaxed text-[15px] ${textColor}`}
              {...props}
            >
              {children}
            </p>
          ),

          // ── Links: underlined + passing contrast ──────────────────────────
          a: ({ children, href, ...props }) => (
            <a
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className={`font-semibold underline decoration-1 underline-offset-2 transition-colors ${linkClass}`}
              {...props}
            >
              {children}
            </a>
          ),

          // ── Code: inline ──────────────────────────────────────────────────
          // light inline code text: #5c3d0a (7.47:1) ✅  — was #6b5d4d (4.16:1)
          code: ({ children, ...props }) => {
            if (inPre) {
              // Inside pre: inherit the pre's text color (text-inherit)
              return (
                <code
                  className="text-[13px] font-mono text-inherit"
                  {...props}
                >
                  {children}
                </code>
              );
            }
            return (
              <code
                className={`inline px-1.5 py-0.5 rounded text-[13px] font-mono ${
                  dark
                    ? 'bg-white/[0.15] text-[#f5c563]'
                    : 'bg-[#e8e0d0] text-[#5c3d0a]'
                }`}
                {...props}
              >
                {children}
              </code>
            );
          },

          // ── Code blocks: fenced / pre ─────────────────────────────────────
          pre: ({ children, ...props }) => (
            <InPreContext.Provider value={true}>
              <pre
                role="region"
                aria-label="Code block"
                className={`mb-4 overflow-x-auto rounded-[12px] p-4 font-mono text-[13px] ${
                  dark
                    ? 'bg-white/[0.12] border border-white/20 text-[#e8dfd0]'
                    : 'bg-white/[0.20] border border-white/30 text-[#2d2820]'
                }`}
                {...props}
              >
                {children}
              </pre>
            </InPreContext.Provider>
          ),

          // ── Lists ─────────────────────────────────────────────────────────
          ul: ({ children, ...props }) => (
            <ul
              className={`list-disc pl-6 mb-4 space-y-1.5 ${textColor}`}
              {...props}
            >
              {children}
            </ul>
          ),
          ol: ({ children, ...props }) => (
            <ol
              className={`list-decimal pl-6 mb-4 space-y-1.5 ${textColor}`}
              {...props}
            >
              {children}
            </ol>
          ),
          li: ({ children, ...props }) => (
            <li
              className={`leading-relaxed ${textColor}`}
              {...props}
            >
              {children}
            </li>
          ),

          // ── Blockquote ────────────────────────────────────────────────────
          blockquote: ({ children, ...props }) => (
            <blockquote
              className={`border-l-4 pl-4 italic my-4 rounded-r-[8px] py-2 ${
                dark
                  ? 'border-[#c9983a]/60 text-[#d4d4d4] bg-white/[0.05]'
                  : 'border-[#c9983a]/70 text-[#4a3f2f] bg-black/[0.04]'
              }`}
              {...props}
            >
              {children}
            </blockquote>
          ),

          // ── Horizontal rule ───────────────────────────────────────────────
          hr: () => (
            <hr
              className={`my-6 border-0 h-px ${dividerColor}`}
              aria-hidden="true"
            />
          ),

          // ── Strong / em ───────────────────────────────────────────────────
          strong: ({ children, ...props }) => (
            <strong className={`font-bold ${headingColor}`} {...props}>
              {children}
            </strong>
          ),
          em: ({ children, ...props }) => (
            <em className={`italic ${textColor}`} {...props}>
              {children}
            </em>
          ),

          // ── Images: lazy, centered, alt passthrough, broken fallback ──────
          img: ({ src, alt, ...props }) => (
            <ReadmeImage
              src={src}
              alt={alt}
              isDark={dark}
              {...props}
            />
          ),

          // ── Tables: new — previously unhandled ────────────────────────────
          table: ({ children, ...props }) => (
            <div className="overflow-x-auto mb-4 rounded-[12px]">
              <table
                role="table"
                className={`w-full text-[14px] border-collapse ${
                  dark ? 'border border-white/10' : 'border border-black/[0.08]'
                }`}
                {...props}
              >
                {children}
              </table>
            </div>
          ),
          thead: ({ children, ...props }) => (
            <thead
              className={tableHeadBg}
              {...props}
            >
              {children}
            </thead>
          ),
          tbody: ({ children, ...props }) => (
            <tbody {...props}>{children}</tbody>
          ),
          tr: ({ children, ...props }) => (
            <tr
              className={`border-b ${tableBorder} odd:bg-transparent even:${tableEvenBg}`}
              {...props}
            >
              {children}
            </tr>
          ),
          th: ({ children, ...props }) => (
            <th
              scope="col"
              className={`px-4 py-2.5 text-left font-semibold text-[13px] border-b ${tableBorder} ${tableHeadText}`}
              {...props}
            >
              {children}
            </th>
          ),
          td: ({ children, ...props }) => (
            <td
              className={`px-4 py-2.5 text-[13px] ${tableCellText}`}
              {...props}
            >
              {children}
            </td>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}

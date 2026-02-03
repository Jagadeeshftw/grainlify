import {
  siTypescript,
  siJavascript,
  siPython,
  siRust,
  siGo,
  siC,
  siHtml5,
  siCplusplus,
  siRuby,
  siPhp,
  siSwift,
  siKotlin,
  siDart,
  siScala,
  siR,
  siShell,
  siPerl,
  siLua,
  siHaskell,
  siClojure,
  siElixir,
  siErlang,
  siElm,
  siFsharp,
  siOcaml,
  siReason,
  siNim,
  siCrystal,
  siZig,
  siV,
  siJulia,
  siFortran,
  siAssemblyscript,
  siWebassembly,
  siDocker,
  siMake,
} from 'simple-icons';

interface LanguageIconProps {
  language: string;
  className?: string;
}

const languageMapping: Record<string, { icon: typeof siTypescript; color: string }> = {
  TypeScript: { icon: siTypescript, color: '#3178C6' },
  JavaScript: { icon: siJavascript, color: '#F7DF1E' },
  Python: { icon: siPython, color: '#3776AB' },
  Rust: { icon: siRust, color: '#CE422B' },
  Go: { icon: siGo, color: '#00ADD8' },
  C: { icon: siC, color: '#A8B9CC' },
  HTML: { icon: siHtml5, color: '#E34F26' },
  'C++': { icon: siCplusplus, color: '#00599C' },
  Ruby: { icon: siRuby, color: '#CC342D' },
  PHP: { icon: siPhp, color: '#777BB4' },
  Swift: { icon: siSwift, color: '#FA7343' },
  Kotlin: { icon: siKotlin, color: '#7F52FF' },
  Dart: { icon: siDart, color: '#0175C2' },
  Scala: { icon: siScala, color: '#DC322F' },
  R: { icon: siR, color: '#276DC3' },
  Shell: { icon: siShell, color: '#89E051' },
  Perl: { icon: siPerl, color: '#39457E' },
  Lua: { icon: siLua, color: '#2C2D72' },
  Haskell: { icon: siHaskell, color: '#5D4F85' },
  Clojure: { icon: siClojure, color: '#5881D8' },
  Elixir: { icon: siElixir, color: '#4B275F' },
  Erlang: { icon: siErlang, color: '#A90533' },
  Elm: { icon: siElm, color: '#1293D8' },
  'F#': { icon: siFsharp, color: '#378BBA' },
  OCaml: { icon: siOcaml, color: '#EC6813' },
  Reason: { icon: siReason, color: '#DD4B39' },
  Nim: { icon: siNim, color: '#FFC200' },
  Crystal: { icon: siCrystal, color: '#000000' },
  Zig: { icon: siZig, color: '#F7A41D' },
  V: { icon: siV, color: '#4F87C4' },
  Julia: { icon: siJulia, color: '#9558B2' },
  Fortran: { icon: siFortran, color: '#734F96' },
  AssemblyScript: { icon: siAssemblyscript, color: '#007AAC' },
  WebAssembly: { icon: siWebassembly, color: '#654FF0' },
  Dockerfile: { icon: siDocker, color: '#2496ED' },
  Makefile: { icon: siMake, color: '#2E7D32' },
  // Languages without icons will show fallback colored dot:
  // CSS, Java, C#, MATLAB, PowerShell, D, COBOL
};

// Custom CSS icon SVG path (CSS3 logo)
const cssIconPath = "M1.5 0h21l-1.91 21.563L11.977 24l-8.564-2.438L1.5 0zm17.09 4.413L5.41 4.41l.213 2.622h10.125l-.255 2.716h-6.64l.24 2.573h6.182l-.366 3.523-2.91.804-2.955-.81-.188-2.11H6.496l.33 4.171L12 19.288l5.379-1.443.744-8.157H8.957l-.276-2.97h10.999z";

// Batchfile (Windows CMD) - terminal-style icon
const batchfileIconPath = "M2 4h20v16H2V4zm2 2v2h16V6H4zm0 4h16v2H4v-2zm0 4h10v2H4v-2z";

export function LanguageIcon({ language, className = 'w-4 h-4' }: LanguageIconProps) {
  const config = languageMapping[language];

  // Special handling for CSS (no icon in simple-icons)
  if (language === 'CSS') {
    return (
      <svg
        role="img"
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
        className={className}
        fill="#1572B6"
      >
        <title>CSS</title>
        <path d={cssIconPath} />
      </svg>
    );
  }

  // Batchfile (Windows CMD) - custom terminal icon
  if (language === 'Batchfile') {
    return (
      <svg
        role="img"
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
        className={className}
        fill="#0078D4"
      >
        <title>Batchfile</title>
        <path d={batchfileIconPath} />
      </svg>
    );
  }

  if (!config) {
    // Fallback to colored dot if language not found
    return <div className={`rounded-full bg-[#9b8b7a] ${className}`} />;
  }

  return (
    <svg
      role="img"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      fill={config.color}
    >
      <title>{language}</title>
      <path d={config.icon.path} />
    </svg>
  );
}

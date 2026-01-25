import {
  siTypescript,
  siJavascript,
  siPython,
  siRust,
  siGo,
  siC,
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
};

export function LanguageIcon({ language, className = 'w-4 h-4' }: LanguageIconProps) {
  const config = languageMapping[language];

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

import { Petal } from '../types';

interface FallingPetalsProps {
  petals: Petal[];
}

export function FallingPetals({ petals }: FallingPetalsProps) {
  return (
    <div className="absolute top-0 left-0 right-0 bottom-0 pointer-events-none z-10 overflow-hidden">
      {petals.map((petal) => (
        <div
          key={petal.id}
          className="absolute -top-10 falling-petal"
          style={{
            left: `${petal.left}%`,
            animationDelay: `${petal.delay}s`,
            animationDuration: `${petal.duration}s`,
            transform: `scale(${petal.size})`,
          }}
        >
          {/* Golden Flower Petal */}
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            style={{
              filter: 'drop-shadow(0 2px 8px rgba(201, 152, 58, 0.4))',
              transform: `rotate(${petal.rotation}deg)`,
            }}
          >
            <path
              d="M12 2 C13 5, 16 7, 19 8 C16 9, 13 11, 12 14 C11 11, 8 9, 5 8 C8 7, 11 5, 12 2 Z"
              fill="url(#goldenGradient)"
              opacity="0.85"
            />
            <defs>
              <linearGradient id="goldenGradient" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#d4af37" />
                <stop offset="50%" stopColor="#c9983a" />
                <stop offset="100%" stopColor="#a67c2e" />
              </linearGradient>
            </defs>
          </svg>
        </div>
      ))}
    </div>
  );
}

export function LeaderboardStyles() {
  return (
    <style>{`
      @keyframes twinkle-slow {
        0%, 100% { opacity: 0.2; transform: scale(1); }
        50% { opacity: 0.8; transform: scale(1.5); }
      }
      
      .animate-twinkle-slow {
        animation: twinkle-slow 4s infinite;
      }
      
      @keyframes glow-pulse {
        0%, 100% { transform: scale(1); opacity: 0.2; }
        50% { transform: scale(1.1); opacity: 0.3; }
      }
      
      .animate-glow-pulse {
        animation: glow-pulse 4s ease-in-out infinite;
      }
      
      .animate-glow-pulse-delayed {
        animation: glow-pulse 4s ease-in-out infinite 2s;
      }
      
      @keyframes float {
        0%, 100% { transform: translateY(0) rotate(0deg); }
        33% { transform: translateY(-20px) rotate(120deg); }
        66% { transform: translateY(-10px) rotate(240deg); }
      }
      
      .animate-float {
        animation: float 8s ease-in-out infinite;
      }
      
      .animate-float-delayed {
        animation: float 8s ease-in-out infinite 2s;
      }
      
      .animate-float-slow {
        animation: float 12s ease-in-out infinite 4s;
      }
      
      @keyframes shimmer {
        0% { transform: translateX(-100%); }
        100% { transform: translateX(100%); }
      }
      
      .animate-shimmer {
        animation: shimmer 3s ease-in-out infinite;
      }
      
      @keyframes bounce-slow {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-10px); }
      }
      
      .animate-bounce-slow {
        animation: bounce-slow 2s ease-in-out infinite;
      }
      
      @keyframes wiggle {
        0%, 100% { transform: rotate(0deg); }
        25% { transform: rotate(-10deg); }
        75% { transform: rotate(10deg); }
      }
      
      .animate-wiggle {
        animation: wiggle 1.5s ease-in-out infinite;
      }
      
      .animate-wiggle-delayed {
        animation: wiggle 1.5s ease-in-out infinite 0.5s;
      }
      
      .animate-wiggle-slow {
        animation: wiggle 3s ease-in-out infinite;
      }
      
      @keyframes pulse-slow {
        0%, 100% { opacity: 1; }
        50% { opacity: 0.6; }
      }
      
      .animate-pulse-slow {
        animation: pulse-slow 2s ease-in-out infinite;
      }
      
      @keyframes ping-slow {
        0%, 100% { transform: scale(1); opacity: 1; }
        50% { transform: scale(1.3); opacity: 0.5; }
      }
      
      .animate-ping-slow {
        animation: ping-slow 2s ease-in-out infinite;
      }
      
      @keyframes pulse-glow {
        0%, 100% { box-shadow: 0 0 20px rgba(201, 152, 58, 0.3); }
        50% { box-shadow: 0 0 40px rgba(201, 152, 58, 0.6); }
      }
      
      .animate-pulse-glow {
        animation: pulse-glow 2s ease-in-out infinite;
      }
      
      @keyframes number-glow {
        0%, 100% { text-shadow: 0 0 10px rgba(201, 152, 58, 0.5); }
        50% { text-shadow: 0 0 20px rgba(201, 152, 58, 0.8), 0 0 30px rgba(201, 152, 58, 0.4); }
      }
      
      .animate-number-glow {
        animation: number-glow 2s ease-in-out infinite;
      }
      
      @keyframes bounce-gentle {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-5px); }
      }
      
      .animate-bounce-gentle {
        animation: bounce-gentle 1s ease-in-out infinite;
      }
      
      @keyframes slide-up {
        from { transform: translateY(10px); opacity: 0; }
        to { transform: translateY(0); opacity: 1; }
      }
      
      .animate-slide-up {
        animation: slide-up 0.5s ease-out 0.6s both;
      }
      
      .animate-slide-up-delayed {
        animation: slide-up 0.5s ease-out 0.7s both;
      }
      
      .animate-slide-up-more-delayed {
        animation: slide-up 0.5s ease-out 0.8s both;
      }
      
      @keyframes pulse-subtle {
        0%, 100% { transform: scale(1); }
        50% { transform: scale(1.02); }
      }
      
      .animate-pulse-subtle {
        animation: pulse-subtle 2s ease-in-out infinite;
      }
      
      @keyframes dropdown-in {
        from { transform: translateY(-10px); opacity: 0; }
        to { transform: translateY(0); opacity: 1; }
      }
      
      .animate-dropdown-in {
        animation: dropdown-in 0.3s ease-out;
      }
      
      @keyframes slideInLeft {
        from { transform: translateX(-30px); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
      }
      
      @keyframes falling {
        0% { transform: translateY(0) rotate(0deg); opacity: 1; }
        100% { transform: translateY(100vh) rotate(360deg); opacity: 0.3; }
      }
      
      .falling-petal {
        animation: falling linear infinite;
      }
      
      @keyframes ray-rotate {
        0%, 100% { transform: rotate(0deg); }
        50% { transform: rotate(360deg); }
      }
      
      .animate-ray-rotate {
        animation: ray-rotate 2s linear infinite;
      }
      
      @keyframes particle-float {
        0%, 100% { transform: translateY(0); opacity: 1; }
        50% { transform: translateY(-20px); opacity: 0.8; }
      }
      
      .animate-particle-float {
        animation: particle-float 3s ease-in-out infinite;
      }
      
      @keyframes ping-gentle {
        0%, 100% { transform: scale(1); opacity: 1; }
        50% { transform: scale(1.1); opacity: 0.8; }
      }
      
      .animate-ping-gentle {
        animation: ping-gentle 2s ease-in-out infinite;
      }
      
      .animate-ping-on-hover {
        animation: ping-on-hover 2s ease-in-out infinite;
      }
    `}</style>
  );
}

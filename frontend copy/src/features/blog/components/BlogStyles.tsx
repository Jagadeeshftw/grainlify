export function BlogStyles() {
  return (
    <style>{`
      @keyframes bounce-slow {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-10px); }
      }
      
      .animate-bounce-slow {
        animation: bounce-slow 3s ease-in-out infinite;
      }
    `}</style>
  );
}

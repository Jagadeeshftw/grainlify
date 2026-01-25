export interface BlogPost {
  id: number;
  title: string;
  excerpt: string;
  content?: string; // Full content for individual post pages
  date: string;
  readTime: string;
  author?: string;
  category?: string;
  icon?: string;
  image?: string;
  isFeatured?: boolean;
}

export interface BlogStatistic {
  icon: React.ReactNode;
  value: string;
  label: string;
}

export interface BlogFeature {
  number: number;
  title: string;
  description: string;
}

export interface BlogWhyChooseCard {
  icon: React.ReactNode;
  title: string;
  description: string;
}

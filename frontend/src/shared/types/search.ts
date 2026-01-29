export interface Project {
  id: string;
  name: string;
  description: string;
}

export interface Issue {
  id: string;
  title: string;
  project: string;
}

export interface Contributor {
  id: string;
  name: string;
  githubHandle: string;
}

export interface SearchResults {
  projects: Project[];
  issues: Issue[];
  contributors: Contributor[];
}

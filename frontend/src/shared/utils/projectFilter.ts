export function getRepoName(githubFullName: string): string {
  // Handles "owner/repo" and edge cases
  const parts = githubFullName.split("/");
  return parts[1] ?? githubFullName;
}

export function isValidProject(project: any): boolean {
  if (!project || !project.id || !project.github_full_name) {
    return false;
  }

  const repoName = getRepoName(project.github_full_name);

  // Exclude special GitHub repositories
  if (repoName === ".github") {
    return false;
  }

  return true;
}

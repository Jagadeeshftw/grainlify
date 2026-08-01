import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ThemeProvider } from '../../../../shared/contexts/ThemeContext';
import { ReadmeEmbed } from '../ReadmeEmbed';

describe('ReadmeEmbed', () => {
  it('shows a loading skeleton when requested', () => {
    const { container } = render(
      <ThemeProvider>
        <ReadmeEmbed content="" theme="light" isLoading />
      </ThemeProvider>,
    );

    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(container.querySelectorAll('.animate-shimmer').length).toBeGreaterThan(0);
  });

  it('renders an empty-state message when there is no README content', () => {
    render(
      <ThemeProvider>
        <ReadmeEmbed content="" theme="light" />
      </ThemeProvider>,
    );

    expect(screen.getByText(/No README content available/i)).toBeInTheDocument();
  });

  it('renders accessible code blocks and tables', () => {
    render(
      <ThemeProvider>
        <ReadmeEmbed
          content={['```ts', 'const answer = 42;', '```', '', '| Name | Value |', '| --- | --- |', '| foo | bar |'].join('\n')}
          theme="dark"
        />
      </ThemeProvider>,
    );

    expect(screen.getByRole('region', { name: /code block/i })).toBeInTheDocument();
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: /name/i })).toBeInTheDocument();
  });

  it('passes image alt text through to the rendered image element', () => {
    render(
      <ThemeProvider>
        <ReadmeEmbed content="![Architecture diagram](https://example.com/arch.png)" theme="light" />
      </ThemeProvider>,
    );

    const image = screen.getByRole('img', { name: /architecture diagram/i });
    expect(image).toHaveAttribute('alt', 'Architecture diagram');
    expect(image).toHaveAttribute('loading', 'lazy');
  });
});

import ReactMarkdown from "react-markdown"

export default function RenderMarkdownContent({
  content,
}: {
  content: string
}) {
  return (
    <ReactMarkdown
      components={{
        h2: ({ children }) => <h2>{children}</h2>,
        p: ({ children }) => <p>{children}</p>,
      }}
    >
      {content}
    </ReactMarkdown>
  )
}

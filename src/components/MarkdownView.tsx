import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import "highlight.js/styles/atom-one-dark.css";

interface Props {
  text: string;
}

const components: Components = {
  pre({ children }) {
    // 외부 <pre> wrapper 제거 — code 컴포넌트가 자체 div+pre 구조를 만듦
    return <>{children}</>;
  },
  code(props) {
    const { className, children } = props;
    const match = /language-(\w+)/.exec(className ?? "");
    if (!match) {
      return <code className={className}>{children}</code>;
    }
    const lang = match[1];
    const text = String(children).replace(/\n$/, "");
    return (
      <div className="md-code">
        <div className="md-code__bar">
          <span className="md-code__lang">{lang}</span>
          <button
            type="button"
            className="md-code__copy"
            onClick={() => {
              void navigator.clipboard.writeText(text);
            }}
          >
            copy
          </button>
        </div>
        <pre className="md-code__pre">
          <code className={className}>{children}</code>
        </pre>
      </div>
    );
  },
};

export default function MarkdownView({ text }: Props) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeHighlight]}
      components={components}
    >
      {text}
    </ReactMarkdown>
  );
}

import React, { useState, useRef, useEffect } from 'react';

interface ReplyComposerProps {
  authorName: string;
  onCancel: () => void;
  onSubmit: (body: string) => Promise<void>;
}

export function ReplyComposer({ authorName, onCancel, onSubmit }: ReplyComposerProps) {
  const [body, setBody] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    textareaRef.current?.focus();
  }, []);

  const handleSubmit = async () => {
    if (!body.trim() || isSubmitting) return;
    setIsSubmitting(true);
    try {
      await onSubmit(body.trim());
      setBody('');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="mt-3 pl-8" role="form" aria-label={`Reply to ${authorName}`}>
      <textarea
        ref={textareaRef}
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder={`Reply to ${authorName}...`}
        aria-label={`Write a reply to ${authorName}`}
        className="w-full min-h-[80px] rounded-[12px] border px-4 py-3 text-[13px] outline-none transition-colors resize-y bg-white/[0.06] border-white/15 text-[#e8dfd0] placeholder:text-[#b8a898]/60 focus-visible:ring-1 focus-visible:ring-[#c9983a]"
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            handleSubmit();
          }
          if (e.key === 'Escape') {
            onCancel();
          }
        }}
      />
      <div className="flex items-center justify-end gap-2 mt-2">
        <button
          type="button"
          onClick={onCancel}
          className="px-3 py-1.5 rounded-[8px] text-[12px] font-semibold bg-white/[0.06] border border-white/10 text-[#d4d4d4] hover:bg-white/[0.1] transition-all focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={!body.trim() || isSubmitting}
          onClick={handleSubmit}
          className="px-3 py-1.5 rounded-[8px] text-[12px] font-semibold bg-gradient-to-br from-[#c9983a] to-[#a67c2e] text-white border border-white/10 hover:opacity-90 transition-all disabled:opacity-50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#c9983a]"
        >
          {isSubmitting ? 'Posting...' : 'Reply'}
        </button>
      </div>
    </div>
  );
}

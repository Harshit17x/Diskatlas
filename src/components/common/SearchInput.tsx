import React from 'react';
import { Search, X } from 'lucide-react';

export interface SearchInputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  value: string;
  onChangeValue?: (val: string) => void;
  shortcut?: string;
  onClear?: () => void;
}

export const SearchInput: React.FC<SearchInputProps> = ({
  value,
  onChangeValue,
  shortcut,
  onClear,
  placeholder = 'Search...',
  className,
  ...props
}) => {
  return (
    <div className={`relative flex items-center w-full ${className || ''}`}>
      <Search className="w-3.5 h-3.5 text-muted absolute left-3 pointer-events-none" />
      <input
        type="text"
        value={value}
        onChange={(e) => onChangeValue && onChangeValue(e.target.value)}
        placeholder={placeholder}
        className="w-full bg-[#14161A] border border-border rounded pl-8 pr-8 py-1.5 text-xs text-primary placeholder-muted focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent transition-colors"
        {...props}
      />
      {value ? (
        <button
          onClick={onClear}
          className="absolute right-2.5 text-muted hover:text-primary transition-colors"
          title="Clear search"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      ) : shortcut ? (
        <span className="absolute right-2.5 text-[10px] font-mono text-muted px-1.5 py-0.5 rounded border border-border bg-panel">
          {shortcut}
        </span>
      ) : null}
    </div>
  );
};


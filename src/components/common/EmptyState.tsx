import React from 'react';
import { Button } from './Button';

export interface EmptyStateProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
  secondaryActionLabel?: string;
  onSecondaryAction?: () => void;
  className?: string;
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  icon,
  title,
  description,
  actionLabel,
  onAction,
  secondaryActionLabel,
  onSecondaryAction,
  className,
}) => {
  return (
    <div
      className={`border border-border border-dashed rounded p-12 flex flex-col items-center justify-center text-center bg-panel/30 max-w-xl mx-auto my-6 ${
        className || ''
      }`}
    >
      <div className="w-12 h-12 rounded bg-elevated border border-border flex items-center justify-center text-muted mb-4">
        {icon}
      </div>
      <h3 className="text-sm font-semibold text-primary">{title}</h3>
      <p className="text-xs text-secondary mt-1.5 max-w-md leading-relaxed">
        {description}
      </p>

      {(actionLabel || secondaryActionLabel) && (
        <div className="flex items-center gap-2.5 mt-5">
          {actionLabel && (
            <Button variant="primary" size="sm" onClick={onAction}>
              {actionLabel}
            </Button>
          )}
          {secondaryActionLabel && (
            <Button variant="secondary" size="sm" onClick={onSecondaryAction}>
              {secondaryActionLabel}
            </Button>
          )}
        </div>
      )}
    </div>
  );
};


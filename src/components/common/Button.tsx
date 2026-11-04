import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'outline';
  size?: 'sm' | 'md' | 'lg';
  icon?: React.ReactNode;
  loading?: boolean;
}

export const Button: React.FC<ButtonProps> = ({
  variant = 'secondary',
  size = 'md',
  icon,
  loading = false,
  children,
  className,
  disabled,
  ...props
}) => {
  const baseStyles =
    'inline-flex items-center justify-center font-medium transition-colors select-none rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-50 disabled:cursor-not-allowed';

  const variantStyles = {
    primary:
      'bg-accent text-white hover:bg-accent-hover active:bg-[#3D6CC2] border border-transparent shadow-sm',
    secondary:
      'bg-panel text-primary hover:bg-elevated hover:text-white border border-border active:bg-[#282D35]',
    danger:
      'bg-danger text-white hover:bg-[#C95353] border border-transparent active:bg-[#B34545]',
    ghost:
      'bg-transparent text-secondary hover:text-primary hover:bg-panel border border-transparent',
    outline:
      'bg-transparent text-primary hover:bg-panel border border-border',
  };

  const sizeStyles = {
    sm: 'text-xs px-2.5 py-1 gap-1.5 h-7',
    md: 'text-xs px-3.5 py-1.5 gap-2 h-8',
    lg: 'text-sm px-4 py-2 gap-2.5 h-10',
  };

  return (
    <button
      className={twMerge(clsx(baseStyles, variantStyles[variant], sizeStyles[size], className))}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? (
        <span className="w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin" />
      ) : (
        icon && <span className="flex-shrink-0">{icon}</span>
      )}
      {children}
    </button>
  );
};

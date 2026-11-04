import React from 'react';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  hoverEffect?: boolean;
}

export const Card: React.FC<CardProps> = ({
  className,
  children,
  hoverEffect = false,
  ...props
}) => {
  return (
    <div
      className={twMerge(
        clsx(
          'bg-slate-900/80 border border-slate-800/80 rounded-xl p-5 shadow-lg backdrop-blur-sm transition-all',
          hoverEffect && 'hover:border-slate-700 hover:bg-slate-900 hover:shadow-slate-950/40 cursor-pointer',
          className
        )
      )}
      {...props}
    >
      {children}
    </div>
  );
};


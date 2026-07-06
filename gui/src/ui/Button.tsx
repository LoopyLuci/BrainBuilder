import { ButtonHTMLAttributes } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: 'sm' | 'md';
}

export function Button({ variant = 'secondary', size = 'sm', className, ...rest }: ButtonProps) {
  const classes = ['bb-btn', `bb-btn--${variant}`, `bb-btn--${size}`, className].filter(Boolean).join(' ');
  return <button className={classes} {...rest} />;
}

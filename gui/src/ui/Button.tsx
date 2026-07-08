import { ButtonHTMLAttributes, forwardRef } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: 'sm' | 'md';
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'secondary', size = 'sm', className, ...rest }, ref) => {
    const classes = ['bb-btn', `bb-btn--${variant}`, `bb-btn--${size}`, className].filter(Boolean).join(' ');
    return <button ref={ref} className={classes} {...rest} />;
  },
);
Button.displayName = 'Button';

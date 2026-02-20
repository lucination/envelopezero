import * as React from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '../../lib/utils'

const badgeVariants = cva('inline-flex items-center rounded-full border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-[0.12em] transition-colors', {
  variants: {
    variant: {
      default: 'border-transparent bg-primary/20 text-primary',
      success: 'border-transparent bg-emerald-500/20 text-emerald-300',
      warning: 'border-transparent bg-amber-500/20 text-amber-300',
      danger: 'border-transparent bg-rose-500/20 text-rose-300',
      outline: 'text-foreground',
    },
  },
  defaultVariants: {
    variant: 'default',
  },
})

export interface BadgeProps extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />
}

export { Badge, badgeVariants }

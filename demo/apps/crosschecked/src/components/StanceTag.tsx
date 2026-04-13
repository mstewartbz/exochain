import { cn } from '@/lib/utils';
import { STANCE_COLORS } from '@/lib/utils';

interface StanceTagProps {
  stance: string;
  size?: 'sm' | 'md';
  className?: string;
}

export default function StanceTag({ stance, size = 'md', className }: StanceTagProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md border font-medium capitalize',
        size === 'sm' ? 'px-1.5 py-0.5 text-[10px]' : 'px-2.5 py-1 text-xs',
        STANCE_COLORS[stance] || 'bg-gray-500/20 text-gray-400 border-gray-400/30',
        className,
      )}
    >
      {stance}
    </span>
  );
}

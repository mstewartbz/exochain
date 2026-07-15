import { cn } from '@/lib/utils';
import { STATUS_COLORS } from '@/lib/utils';

interface StatusBadgeProps {
  status: string;
  className?: string;
}

export default function StatusBadge({ status, className }: StatusBadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium capitalize',
        STATUS_COLORS[status] || 'bg-gray-500/20 text-gray-400',
        className,
      )}
    >
      {status}
    </span>
  );
}

import { PANELS, PANEL_COLORS } from '@/lib/utils';
import { cn } from '@/lib/utils';
import OpinionCard from '@/components/OpinionCard';

interface Opinion {
  agent_did: string;
  agent_label?: string;
  model?: string;
  stance: string;
  summary: string;
  rationale?: string;
  confidence?: number;
  risks?: string[];
  panel?: string;
  property?: string;
}

interface PanelGridProps {
  opinions: Opinion[];
}

export default function PanelGrid({ opinions }: PanelGridProps) {
  const grouped = PANELS.reduce(
    (acc, panel) => {
      acc[panel] = opinions.filter((o) => o.panel === panel);
      return acc;
    },
    {} as Record<string, Opinion[]>,
  );

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-5 gap-4">
      {PANELS.map((panel) => (
        <div key={panel} className="space-y-3">
          <div className="flex items-center gap-2 mb-2">
            <span
              className={cn(
                'text-xs px-2 py-0.5 rounded font-medium',
                PANEL_COLORS[panel],
              )}
            >
              {panel}
            </span>
            <span className="text-xs text-gray-500">{grouped[panel].length}</span>
          </div>
          {grouped[panel].length === 0 ? (
            <div className="rounded-lg border border-dashed border-white/10 p-4 text-center text-xs text-gray-500">
              No opinions
            </div>
          ) : (
            grouped[panel].map((op, i) => (
              <OpinionCard key={`${op.agent_did}-${i}`} opinion={op} compact />
            ))
          )}
        </div>
      ))}
    </div>
  );
}

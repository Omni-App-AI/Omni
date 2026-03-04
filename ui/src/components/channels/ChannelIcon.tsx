import {
  MessageCircle,
  Send,
  Phone,
  Hash,
  MessageSquare,
  Users,
  Terminal,
  Tv,
  Globe,
  Zap,
  Cloud,
  Server,
  Smartphone,
  Shield,
  AtSign,
  Radio,
} from "lucide-react";
import type { LucideProps } from "lucide-react";

const ICON_MAP: Record<string, React.ComponentType<LucideProps>> = {
  MessageCircle,
  Send,
  Phone,
  Hash,
  MessageSquare,
  Users,
  Terminal,
  Tv,
  Globe,
  Zap,
  Cloud,
  Server,
  Smartphone,
  Shield,
  AtSign,
  Radio,
};

interface ChannelIconProps extends LucideProps {
  iconName: string;
}

export function ChannelIcon({ iconName, ...props }: ChannelIconProps) {
  const Icon = ICON_MAP[iconName] ?? Radio;
  return <Icon {...props} />;
}

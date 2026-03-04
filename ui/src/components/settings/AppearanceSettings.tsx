import { ThemeSection } from "./appearance/ThemeSection";
import { AccentColorSection } from "./appearance/AccentColorSection";
import { TypographySection } from "./appearance/TypographySection";
import { LayoutDensitySection } from "./appearance/LayoutDensitySection";
import { ChatAppearanceSection } from "./appearance/ChatAppearanceSection";
import { BordersShapesSection } from "./appearance/BordersShapesSection";
import { MotionAccessibilitySection } from "./appearance/MotionAccessibilitySection";
import { ResetSection } from "./appearance/ResetSection";

export function AppearanceSettings() {
  return (
    <div className="space-y-6">
      <ThemeSection />
      <AccentColorSection />
      <TypographySection />
      <LayoutDensitySection />
      <ChatAppearanceSection />
      <BordersShapesSection />
      <MotionAccessibilitySection />
      <ResetSection />
    </div>
  );
}

/**
 * Fork configuration gates for the onboarding product layer.
 *
 * The Lunar Park fork ships without Buzz's bundled welcome experience — the
 * private Welcome channel, the welcome team/canvas seeding, and the kickoff
 * conversation it drives. The upstream code paths stay intact so rebases stay
 * cheap; this gate only controls whether they run.
 *
 * Set `VITE_BUZZ_WELCOME_EXPERIENCE=1` (or `true`) at build time to restore
 * the upstream behavior.
 */
export function welcomeExperienceEnabled(
  value: string | undefined = import.meta.env?.VITE_BUZZ_WELCOME_EXPERIENCE as
    | string
    | undefined,
): boolean {
  const normalized = value?.trim();
  return normalized === "1" || normalized === "true";
}

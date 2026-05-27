export interface PasswordStrength {
  bits: number;
  level: "weak" | "fair" | "good" | "strong" | "excellent";
  color: string;
  gradient: string;
  textColor: string;
}

function calculateEntropy(password: string): number {
  if (!password || password.length === 0) return 0;

  let charSpace = 0;
  if (/[a-z]/.test(password)) charSpace += 26;
  if (/[A-Z]/.test(password)) charSpace += 26;
  if (/[0-9]/.test(password)) charSpace += 10;
  if (/[^a-zA-Z0-9]/.test(password)) charSpace += 33;
  if (charSpace === 0) return 0;

  let entropy = password.length * Math.log2(charSpace);

  const charCounts = new Map<string, number>();
  for (const char of password) {
    charCounts.set(char, (charCounts.get(char) || 0) + 1);
  }

  let repetitionPenalty = 0;
  for (const count of charCounts.values()) {
    if (count > 1) repetitionPenalty += (count - 1) * 2;
  }

  let sequenceCount = 0;
  for (let i = 0; i < password.length - 2; i++) {
    const c1 = password.charCodeAt(i);
    const c2 = password.charCodeAt(i + 1);
    const c3 = password.charCodeAt(i + 2);
    if ((c2 === c1 + 1 && c3 === c2 + 1) || (c2 === c1 - 1 && c3 === c2 - 1)) {
      sequenceCount++;
    }
  }
  const sequencePenalty = sequenceCount * 3;

  const commonPatterns = ["qwerty", "asdf", "zxcv", "1234", "abcd"];
  let patternPenalty = 0;
  const lower = password.toLowerCase();
  for (const pattern of commonPatterns) {
    if (lower.includes(pattern)) patternPenalty += 8;
  }

  return Math.max(0, entropy - repetitionPenalty - sequencePenalty - patternPenalty);
}

export function calculatePasswordStrength(password: string): PasswordStrength {
  const bits = Math.round(calculateEntropy(password));

  let level: PasswordStrength["level"];
  if (bits < 40) level = "weak";
  else if (bits < 64) level = "fair";
  else if (bits < 80) level = "good";
  else if (bits < 112) level = "strong";
  else level = "excellent";

  const ratio = Math.min(bits / 128, 1);
  const hue = ratio * 120;
  const saturation = 70;
  const lightness = 50;

  const gradientColor = `hsl(${hue}, ${saturation}%, ${lightness}%)`;
  const gradientColorDark = `hsl(${hue}, ${saturation}%, ${lightness - 10}%)`;
  const gradient = `linear-gradient(90deg, ${gradientColor}, ${gradientColorDark})`;

  return {
    bits,
    level,
    color: gradientColor,
    gradient,
    textColor: gradientColor,
  };
}

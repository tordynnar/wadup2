interface GopherIconProps {
  size?: number
}

export default function GopherIcon({ size = 24 }: GopherIconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 400 560"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Body */}
      <ellipse cx="200" cy="300" rx="160" ry="220" fill="#6AD7E5" stroke="#000" strokeWidth="8"/>

      {/* Left ear */}
      <ellipse cx="70" cy="80" rx="35" ry="45" fill="#6AD7E5" stroke="#000" strokeWidth="6"/>
      <ellipse cx="70" cy="80" rx="18" ry="25" fill="#F6D2A2"/>

      {/* Right ear */}
      <ellipse cx="330" cy="80" rx="35" ry="45" fill="#6AD7E5" stroke="#000" strokeWidth="6"/>
      <ellipse cx="330" cy="80" rx="18" ry="25" fill="#F6D2A2"/>

      {/* Left eye white */}
      <ellipse cx="130" cy="180" rx="55" ry="60" fill="#fff" stroke="#000" strokeWidth="6"/>
      {/* Left pupil */}
      <ellipse cx="140" cy="185" rx="20" ry="22" fill="#000"/>
      <ellipse cx="147" cy="178" rx="5" ry="6" fill="#fff"/>

      {/* Right eye white */}
      <ellipse cx="270" cy="180" rx="55" ry="60" fill="#fff" stroke="#000" strokeWidth="6"/>
      {/* Right pupil */}
      <ellipse cx="280" cy="185" rx="20" ry="22" fill="#000"/>
      <ellipse cx="287" cy="178" rx="5" ry="6" fill="#fff"/>

      {/* Nose/snout */}
      <ellipse cx="200" cy="270" rx="45" ry="35" fill="#F6D2A2" stroke="#000" strokeWidth="5"/>
      <ellipse cx="200" cy="260" rx="25" ry="15" fill="#4a3728"/>

      {/* Tooth */}
      <rect x="188" y="285" width="24" height="30" rx="4" fill="#fff" stroke="#000" strokeWidth="3"/>

      {/* Left hand */}
      <ellipse cx="50" cy="380" rx="30" ry="45" fill="#F6D2A2" stroke="#000" strokeWidth="5"/>

      {/* Right hand */}
      <ellipse cx="350" cy="380" rx="30" ry="45" fill="#F6D2A2" stroke="#000" strokeWidth="5"/>

      {/* Left foot */}
      <ellipse cx="130" cy="510" rx="50" ry="30" fill="#F6D2A2" stroke="#000" strokeWidth="5"/>

      {/* Right foot */}
      <ellipse cx="270" cy="510" rx="50" ry="30" fill="#F6D2A2" stroke="#000" strokeWidth="5"/>
    </svg>
  )
}

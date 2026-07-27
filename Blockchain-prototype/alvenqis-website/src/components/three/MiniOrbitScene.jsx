import { Float, Line, Stars } from '@react-three/drei'
import { Canvas, useFrame } from '@react-three/fiber'
import { useMemo, useRef } from 'react'
import * as THREE from 'three'

function Rig({ variant = 'core' }) {
  const ref = useRef()
  const colors = {
    core: ['#38bdf8', '#7dd3fc', '#a78bfa'],
    mining: ['#7dd3fc', '#eef7ff', '#8b5cf6'],
    wallet: ['#a78bfa', '#38bdf8', '#eef7ff'],
    explorer: ['#38bdf8', '#8b5cf6', '#7dd3fc'],
  }[variant] || ['#38bdf8', '#7dd3fc', '#a78bfa']

  const points = useMemo(() => {
    const curve = new THREE.EllipseCurve(0, 0, 2.15, 0.72, 0, Math.PI * 2, false, 0)
    return curve.getPoints(80).map((point) => new THREE.Vector3(point.x, 0, point.y))
  }, [])

  useFrame((state) => {
    if (!ref.current) return
    const time = state.clock.getElapsedTime()
    ref.current.rotation.y = time * 0.24
    ref.current.rotation.x = Math.sin(time * 0.25) * 0.12
  })

  return (
    <Float speed={1.1} rotationIntensity={0.15} floatIntensity={0.35}>
      <group ref={ref}>
        <mesh>
          <octahedronGeometry args={[0.9, 1]} />
          <meshStandardMaterial color="#11152b" emissive={colors[0]} emissiveIntensity={1.1} metalness={0.7} roughness={0.18} wireframe />
        </mesh>
        <mesh>
          <boxGeometry args={[0.58, 0.58, 0.58]} />
          <meshStandardMaterial color={colors[1]} emissive={colors[1]} emissiveIntensity={1.25} metalness={0.42} roughness={0.2} transparent opacity={0.82} />
        </mesh>
        <group rotation={[0.45, 0, 0]}>
          <Line points={points} color={colors[2]} transparent opacity={0.42} lineWidth={1} />
        </group>
        {[0, 1.7, 3.4].map((offset, index) => (
          <mesh key={offset} position={[Math.cos(offset) * 2.15, Math.sin(offset * 0.6) * 0.35, Math.sin(offset) * 0.72]}>
            <boxGeometry args={[0.14 + index * 0.02, 0.14 + index * 0.02, 0.14 + index * 0.02]} />
            <meshStandardMaterial color={colors[index]} emissive={colors[index]} emissiveIntensity={1.4} />
          </mesh>
        ))}
      </group>
    </Float>
  )
}

export default function MiniOrbitScene({ variant = 'core' }) {
  return (
    <div className="absolute inset-0 overflow-hidden">
      <Canvas camera={{ position: [0, 0.1, 4.8], fov: 45 }} dpr={[1, 1.5]}>
        <color attach="background" args={['#05060d']} />
        <ambientLight intensity={0.55} />
        <pointLight position={[3, 4, 3]} color="#38bdf8" intensity={34} />
        <pointLight position={[-3, -2, -3]} color="#a78bfa" intensity={22} />
        <Stars radius={18} depth={8} count={360} factor={1.5} saturation={0} fade speed={0.35} />
        <Rig variant={variant} />
      </Canvas>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_12%,rgba(5,6,13,0.28)_54%,rgba(5,6,13,0.86)_100%)]" />
    </div>
  )
}

import { Float, Line, OrbitControls, Stars } from '@react-three/drei'
import { Canvas, useFrame } from '@react-three/fiber'
import { useMemo, useRef } from 'react'
import * as THREE from 'three'

function Core() {
  const groupRef = useRef()
  const innerRef = useRef()
  const ringRef = useRef()

  useFrame((state) => {
    if (!groupRef.current || !ringRef.current || !innerRef.current) return
    const t = state.clock.getElapsedTime()
    const pulse = 1 + Math.sin(t * 1.35) * 0.055
    innerRef.current.scale.setScalar(pulse)
    innerRef.current.rotation.y -= 0.005
    groupRef.current.rotation.y = t * 0.17
    groupRef.current.rotation.x = Math.sin(t * 0.26) * 0.09
    ringRef.current.rotation.z = -t * 0.28
  })

  return (
    <group ref={groupRef}>
      <mesh>
        <icosahedronGeometry args={[1.55, 1]} />
        <meshStandardMaterial
          color="#11152b"
          emissive="#38bdf8"
          emissiveIntensity={0.72}
          metalness={0.72}
          roughness={0.22}
          wireframe
        />
      </mesh>
      <mesh ref={innerRef}>
        <icosahedronGeometry args={[0.82, 1]} />
        <meshStandardMaterial
          color="#7dd3fc"
          emissive="#7dd3fc"
          emissiveIntensity={1.8}
          metalness={0.62}
          roughness={0.18}
          transparent
          opacity={0.9}
        />
      </mesh>
      <mesh ref={ringRef} rotation={[Math.PI / 2.4, 0, 0]}>
        <torusGeometry args={[2.58, 0.01, 16, 180]} />
        <meshBasicMaterial color="#7dd3fc" transparent opacity={0.42} />
      </mesh>
      <mesh rotation={[0.6, 0.1, 0.8]}>
        <torusGeometry args={[3.16, 0.008, 16, 200]} />
        <meshBasicMaterial color="#a78bfa" transparent opacity={0.32} />
      </mesh>
      <mesh rotation={[1.2, 0.35, 1.2]}>
        <torusGeometry args={[3.62, 0.006, 16, 220]} />
        <meshBasicMaterial color="#38bdf8" transparent opacity={0.18} />
      </mesh>
      <pointLight color="#38bdf8" intensity={10} distance={7} />
      <pointLight color="#a78bfa" intensity={5} distance={5} position={[0, 0, 0]} />
    </group>
  )
}

function OrbitBlock({ radius, speed, offset, size, color }) {
  const ref = useRef()

  useFrame((state) => {
    if (!ref.current) return
    const t = state.clock.getElapsedTime() * speed + offset
    ref.current.position.set(
      Math.cos(t) * radius,
      Math.sin(t * 0.62) * radius * 0.34,
      Math.sin(t) * radius,
    )
    ref.current.rotation.x += 0.012
    ref.current.rotation.y += 0.018
  })

  return (
    <mesh ref={ref}>
      <boxGeometry args={[size, size, size]} />
      <meshStandardMaterial color={color} emissive={color} emissiveIntensity={1.35} metalness={0.45} roughness={0.26} />
    </mesh>
  )
}

function ArcTrail({ radius, color, tilt = 0, speed = 0.4 }) {
  const ref = useRef()
  const points = useMemo(() => {
    const curve = new THREE.EllipseCurve(0, 0, radius, radius * 0.42, 0, Math.PI * 2, false, 0)
    return curve.getPoints(86).map((point) => new THREE.Vector3(point.x, 0, point.y))
  }, [radius])

  useFrame((state) => {
    if (!ref.current) return
    ref.current.rotation.y = state.clock.getElapsedTime() * speed * 0.08
    ref.current.rotation.x = 0.28 + tilt
  })

  return (
    <group ref={ref}>
      <Line points={points} color={color} transparent opacity={0.26} lineWidth={1} />
    </group>
  )
}

function Particles() {
  const particles = useMemo(() => {
    const result = []
    for (let i = 0; i < 120; i += 1) {
      const radius = 4.2 + Math.random() * 4.8
      const theta = Math.random() * Math.PI * 2
      const y = (Math.random() - 0.5) * 4.2
      result.push(new THREE.Vector3(Math.cos(theta) * radius, y, Math.sin(theta) * radius))
    }
    return result
  }, [])

  return (
    <group>
      {particles.map((point, index) => (
        <mesh key={`${point.x}-${index}`} position={point}>
          <sphereGeometry args={[0.015 + (index % 4) * 0.003, 8, 8]} />
          <meshBasicMaterial color={index % 3 === 0 ? '#a78bfa' : '#7dd3fc'} transparent opacity={0.48} />
        </mesh>
      ))}
    </group>
  )
}

function Scene() {
  return (
    <>
      <color attach="background" args={['#05060d']} />
      <fog attach="fog" args={['#05060d', 4, 10]} />
      <ambientLight intensity={0.8} />
      <pointLight position={[4, 5, 3]} color="#38bdf8" intensity={72} />
      <pointLight position={[-4, -2, -4]} color="#a78bfa" intensity={36} />
      <Stars radius={30} depth={18} count={1100} factor={2.2} saturation={0} fade speed={0.42} />
      <Float speed={1.25} rotationIntensity={0.16} floatIntensity={0.46}>
        <Core />
        <ArcTrail radius={2.7} color="#38bdf8" speed={0.7} />
        <ArcTrail radius={3.3} color="#a78bfa" tilt={0.22} speed={-0.45} />
        <ArcTrail radius={3.8} color="#7dd3fc" tilt={-0.18} speed={0.35} />
        <OrbitBlock radius={2.7} speed={0.45} offset={0} size={0.15} color="#38bdf8" />
        <OrbitBlock radius={3.3} speed={-0.32} offset={2.1} size={0.12} color="#a78bfa" />
        <OrbitBlock radius={3.8} speed={0.38} offset={4.2} size={0.17} color="#7dd3fc" />
        <OrbitBlock radius={3.5} speed={-0.25} offset={1.4} size={0.11} color="#eef7ff" />
      </Float>
      <Particles />
      <OrbitControls enablePan={false} enableZoom={false} autoRotate autoRotateSpeed={0.55} />
    </>
  )
}

export default function HeroScene() {
  return (
    <div className="absolute inset-0 overflow-hidden bg-radial">
      <Canvas camera={{ position: [0, 0.25, 7], fov: 45 }} dpr={[1, 1.8]}>
        <Scene />
      </Canvas>
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,transparent_26%,rgba(5,6,13,0.08)_56%,rgba(5,6,13,0.92)_100%)]" />
    </div>
  )
}

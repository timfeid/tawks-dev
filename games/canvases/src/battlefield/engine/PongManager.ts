import * as THREE from 'three';

export class PongManager {
  private scene: THREE.Scene;
  private group: THREE.Group;

  private radius = 8.5;
  private ball: THREE.Mesh;
  private paddles: Map<string, THREE.Mesh> = new Map();
  private field: THREE.Mesh;

  private ballPos = new THREE.Vector2(0, 0);
  private ballVelocity = new THREE.Vector2(0, 0);
  private isActive = false;

  private paddleWidth = 1.5;
  private paddleThickness = 0.15;
  private paddleHeight = 0.05;
  private paddleRadius = 8.3;

  constructor(scene: THREE.Scene) {
    this.scene = scene;
    this.group = new THREE.Group();
    this.group.position.y = 0.02;

    const fieldGeo = new THREE.RingGeometry(this.radius - 0.05, this.radius, 128);
    const fieldMat = new THREE.MeshBasicMaterial({
      color: 0xf59e0b,
      transparent: true,
      opacity: 0.4,
      side: THREE.DoubleSide,
    });
    this.field = new THREE.Mesh(fieldGeo, fieldMat);
    this.field.rotation.x = -Math.PI / 2;
    this.group.add(this.field);

    const grooveGeo = new THREE.RingGeometry(this.paddleRadius - 0.2, this.paddleRadius + 0.2, 128);
    const grooveMat = new THREE.MeshBasicMaterial({
      color: 0x000000,
      transparent: true,
      opacity: 0.3,
      side: THREE.DoubleSide,
    });
    const groove = new THREE.Mesh(grooveGeo, grooveMat);
    groove.rotation.x = -Math.PI / 2;
    groove.position.y = -0.01;
    this.group.add(groove);

    const ballGeo = new THREE.CylinderGeometry(0.25, 0.25, 0.05, 32);
    const ballMat = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      emissive: 0xffffff,
      emissiveIntensity: 2.0,
    });
    this.ball = new THREE.Mesh(ballGeo, ballMat);
    this.group.add(this.ball);
  }

  public setVisible(visible: boolean) {
    if (visible && !this.isActive) {
      this.scene.add(this.group);
      this.isActive = true;
      this.resetBall();
    } else if (!visible && this.isActive) {
      this.scene.remove(this.group);
      this.isActive = false;
    }
  }

  private resetBall() {
    this.ballPos.set(0, 0);
    const angle = Math.random() * Math.PI * 2;
    const speed = 6.0 + Math.random() * 4.0;
    this.ballVelocity.set(Math.cos(angle) * speed, Math.sin(angle) * speed);
  }

  public update(
    delta: number,
    players: { id: string }[],
    viewerId: string | null,
    moveLeft: boolean,
    moveRight: boolean
  ) {
    if (!this.isActive) return;

    // 1. Sync Paddles (1v1 vs NPC)
    this.syncPaddles();

    // 2. Local Player Movement
    const playerPaddle = this.paddles.get('player');
    if (playerPaddle) {
        const moveSpeed = 3.0;
        if (moveLeft) playerPaddle.userData.angle += moveSpeed * delta;
        if (moveRight) playerPaddle.userData.angle -= moveSpeed * delta;
        this.clampPaddle(playerPaddle, 0, Math.PI); // Half circle limit
        playerPaddle.rotation.z = -playerPaddle.userData.angle;
    }

    // 3. NPC Movement
    const npcPaddle = this.paddles.get('npc');
    if (npcPaddle) {
        const ballAngle = Math.atan2(this.ballPos.y, this.ballPos.x);
        let diff = ballAngle - npcPaddle.userData.angle;
        while (diff > Math.PI) diff -= Math.PI * 2;
        while (diff < -Math.PI) diff += Math.PI * 2;

        const npcSpeed = 2.0;
        const step = Math.sign(diff) * Math.min(Math.abs(diff), npcSpeed * delta);
        npcPaddle.userData.angle += step;
        this.clampPaddle(npcPaddle, Math.PI, Math.PI); // Other half
        npcPaddle.rotation.z = -npcPaddle.userData.angle;
    }

    // 4. Move Ball
    this.ballPos.x += this.ballVelocity.x * delta;
    this.ballPos.y += this.ballVelocity.y * delta;
    this.ball.position.x = this.ballPos.x;
    this.ball.position.z = this.ballPos.y;

    // 5. Physics & Feedback
    this.handleCollisions(delta);
  }

  private handleCollisions(delta: number) {
    const dist = this.ballPos.length();
    const ballRadius = 0.25;

    for (const paddle of this.paddles.values()) {
      const mat = paddle.material as THREE.MeshStandardMaterial;
      if (mat.emissiveIntensity > 2.0) mat.emissiveIntensity -= delta * 5.0;

      const paddleAngle = paddle.userData.angle;
      let diff = Math.atan2(this.ballPos.y, this.ballPos.x) - paddleAngle;
      while (diff > Math.PI) diff -= Math.PI * 2;
      while (diff < -Math.PI) diff += Math.PI * 2;

      const angularWidth = (this.paddleWidth + 0.4) / this.paddleRadius;
      if (dist > this.paddleRadius - ballRadius - 0.2 && Math.abs(diff) < angularWidth / 2) {
        const normal = new THREE.Vector2(Math.cos(paddleAngle), Math.sin(paddleAngle)).negate();
        const dot = this.ballVelocity.dot(normal);

        if (dot < 0) {
          const reflectionPart = normal.clone().multiplyScalar(2 * dot);
          this.ballVelocity.sub(reflectionPart);
          this.ballVelocity.multiplyScalar(1.05);

          const hitFactor = diff / (angularWidth / 2);
          const tangent = new THREE.Vector2(-normal.y, normal.x); 
          this.ballVelocity.add(tangent.multiplyScalar(hitFactor * 4));

          mat.emissiveIntensity = 6.0;
          this.ballPos.add(normal.clone().multiplyScalar(-0.4));
        }
      }
    }

    if (dist > this.radius + 0.5) {
      this.resetBall();
    }
  }

  private clampPaddle(paddle: THREE.Mesh, startAngle: number, range: number) {
      let diff = paddle.userData.angle - startAngle;
      while (diff > Math.PI) diff -= Math.PI * 2;
      while (diff < -Math.PI) diff += Math.PI * 2;
      diff = THREE.MathUtils.clamp(diff, -range/2 + 0.2, range/2 - 0.2);
      paddle.userData.angle = startAngle + diff;
  }

  private syncPaddles() {
    if (this.paddles.size === 0) {
        this.createPaddle('player', 0);
        this.createPaddle('npc', Math.PI);
    }
  }

  private createPaddle(id: string, angle: number) {
    const inner = this.paddleRadius - this.paddleThickness / 2;
    const outer = this.paddleRadius + this.paddleThickness / 2;
    const thetaSize = this.paddleWidth / this.paddleRadius;
    const paddleGeo = new THREE.RingGeometry(inner, outer, 32, 1, -thetaSize / 2, thetaSize);
    const paddleMat = new THREE.MeshStandardMaterial({
      color: 0xfacc15,
      emissive: 0xfacc15,
      emissiveIntensity: 2.0,
      transparent: true,
      opacity: 0.9,
      side: THREE.DoubleSide,
    });
    const paddle = new THREE.Mesh(paddleGeo, paddleMat);
    paddle.rotation.x = -Math.PI / 2;
    paddle.userData.angle = angle;
    this.group.add(paddle);
    this.paddles.set(id, paddle);
  }
}

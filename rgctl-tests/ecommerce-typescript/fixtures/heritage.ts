export interface IService {
  run(): void;
}

export interface IOrderService extends IService {
  checkout(): void;
}

export class OrderServiceImpl implements IOrderService {
  run(): void {}
  checkout(): void {}
}

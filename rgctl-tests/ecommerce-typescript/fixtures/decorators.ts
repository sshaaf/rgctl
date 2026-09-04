function Controller(path: string) {
  return (target: unknown) => target;
}

function Get() {
  return (target: unknown) => target;
}

@Controller('orders')
export class OrdersControllerFixture {
  @Get()
  list(): string {
    return 'ok';
  }
}

// Inheritance fixture for C++ extraction-depth gates (Extends / template bases).
namespace ecommerce::fixtures {

class BaseService {
public:
    virtual void serve() = 0;
};

class OtherBase {
public:
    void helper() {}
};

class HiddenBase {};

class ProductServiceImpl : public BaseService {
public:
    void serve() override {}
};

class MultiBase : public BaseService, protected OtherBase, private HiddenBase {};

template <typename T>
class TemplateDerived : public BaseService, public OtherBase {};

}  // namespace ecommerce::fixtures
